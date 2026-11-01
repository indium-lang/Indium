use crate::{
    ast::*,
    diagnostic::{Diagnostic, Result, Span},
    loader::{resolve_import, Program},
    runtime::{
        environment::{coerce, Env, Environment},
        values::{Function, Value},
    },
    stdlib,
};
use std::{collections::BTreeMap, path::Path, rc::Rc, time::Instant};

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub capture_output: bool,
    pub max_steps: Option<u64>,
    pub max_frames: Option<u64>,
}
#[derive(Clone)]
pub struct Handler {
    pub event: Event,
    pub body: Rc<Vec<Stmt>>,
    pub env: Env,
    pub file: String,
    pub interval: f64,
    pub next: f64,
}
#[derive(Debug, Clone)]
pub enum Flow {
    Normal,
    Return(Value),
    Break,
    Continue,
}
pub struct Interpreter {
    pub program: Program,
    pub globals: Env,
    pub options: Options,
    pub output: String,
    pub file: String,
    pub handlers: Vec<Handler>,
    pub stopped: bool,
    pub started: Instant,
    pub frame_dt: f64,
    pub tick_dt: f64,
    pub tick_count: i64,
    pub frame_count: i64,
    pub tps: f64,
    pub max_catchup: usize,
    pub dropped_ticks: u64,
    pub args: Vec<String>,
    pub rng: u64,
    pub media: crate::media::Media,
    modules: BTreeMap<String, Env>,
    steps: u64,
    call_depth: usize,
    eval_depth: usize,
    block_depth: usize,
}
impl Interpreter {
    pub fn new(program: Program, options: Options) -> Self {
        let globals = Environment::new(None);
        for (name, value) in stdlib::initial_values() {
            Environment::define(&globals, &name, value, true);
        }
        Environment::define(&globals, "mode", Value::Mode("terminal".into()), false);
        let file = program.entry.clone();
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(49, |d| d.as_nanos() as u64)
            | 1;
        Self {
            program,
            globals,
            options,
            output: String::new(),
            file,
            handlers: Vec::new(),
            stopped: false,
            started: Instant::now(),
            frame_dt: 0.0,
            tick_dt: 1.0 / 240.0,
            tick_count: 0,
            frame_count: 0,
            tps: 240.0,
            max_catchup: 8,
            dropped_ticks: 0,
            args: Vec::new(),
            rng: seed,
            media: crate::media::Media::default(),
            modules: BTreeMap::new(),
            steps: 0,
            call_depth: 0,
            eval_depth: 0,
            block_depth: 0,
        }
    }
    pub fn run(&mut self) -> Result<()> {
        let body = self.program.modules[&self.program.entry].clone();
        let env = self.globals.clone();
        self.execute(&body, &env)?;
        if self.stopped {
            return Ok(());
        }
        let mode =
            Environment::get(&self.globals, "mode").unwrap_or(Value::Mode("terminal".into()));
        match mode {
            Value::Mode(ref m) if m == "3d" => {
                return Err(self.error("3d mode is not implemented yet", Span::default()))
            }
            Value::Mode(ref m) if m == "2d" => {
                self.media
                    .open_window()
                    .map_err(|e| self.error(e, Span::default()))?;
            }
            Value::Mode(ref m) if m == "terminal" || m == "background" => {}
            _ => {
                return Err(self.error(
                    "mode must be terminal, background, 2d, or 3d",
                    Span::default(),
                ))
            }
        }
        crate::runtime::scheduler::run(self)
    }
    pub fn mode_2d(&self) -> bool {
        matches!(Environment::get(&self.globals,"mode"),Some(Value::Mode(m)) if m=="2d")
    }
    pub fn error(&self, message: impl Into<String>, span: Span) -> Diagnostic {
        Diagnostic::new(message, span).in_file(&self.file)
    }
    fn step(&mut self, span: Span) -> Result<()> {
        self.steps = self.steps.saturating_add(1);
        if self.options.max_steps.is_some_and(|n| self.steps > n) {
            return Err(self.error("Execution step limit reached", span));
        }
        Ok(())
    }
    pub fn execute(&mut self, body: &[Stmt], env: &Env) -> Result<Flow> {
        if self.block_depth >= 32 {
            let span = body.first().map_or(Span::default(), |s| s.span);
            return Err(self.error("Maximum execution block depth reached (32)", span));
        }
        self.block_depth += 1;
        let result = self.execute_inner(body, env);
        self.block_depth -= 1;
        result
    }
    fn execute_inner(&mut self, body: &[Stmt], env: &Env) -> Result<Flow> {
        for s in body {
            if let StmtKind::Function {
                name,
                params,
                result,
                body,
            } = &s.kind
            {
                let f = Function {
                    name: name.clone(),
                    params: params.clone(),
                    result: result.clone(),
                    body: body.clone(),
                    closure: env.clone(),
                    file: self.file.clone(),
                };
                Environment::define(env, name, Value::Function(Rc::new(f)), true);
            }
        }
        for s in body {
            if self.stopped {
                return Ok(Flow::Normal);
            }
            self.step(s.span)?;
            let flow = self.statement(s, env).map_err(|e| e.in_file(&self.file))?;
            if !matches!(flow, Flow::Normal) {
                return Ok(flow);
            }
        }
        Ok(Flow::Normal)
    }
    fn scoped(&mut self, body: &[Stmt], parent: &Env) -> Result<Flow> {
        self.execute(body, &Environment::new(Some(parent.clone())))
    }
    fn statement(&mut self, s: &Stmt, env: &Env) -> Result<Flow> {
        match &s.kind {
            StmtKind::Function { .. } => {}
            StmtKind::Expression(e) => {
                self.eval(e, env)?;
            }
            StmtKind::Assign {
                target,
                ty,
                op,
                value,
            } => {
                // Evaluate an indexed/member target once, before its right-hand side.
                let location = self.location(target, env)?;
                let previous = if op.is_some() {
                    Some(self.read_location(&location, target.span)?)
                } else {
                    None
                };
                let mut v = self.eval(value, env)?;
                if let Some(op) = op {
                    v = self.binary(previous.unwrap(), *op, v, s.span)?;
                }
                self.write_location(location, v, ty.as_ref(), s.span)?;
            }
            StmtKind::If {
                condition,
                then_body,
                else_body,
            } => {
                let c = self.eval(condition, env)?;
                return if self.boolean(c, condition.span)? {
                    self.scoped(then_body, env)
                } else {
                    self.scoped(else_body, env)
                };
            }
            StmtKind::While { condition, body } => loop {
                if self.stopped {
                    break;
                }
                let c = self.eval(condition, env)?;
                if !self.boolean(c, condition.span)? {
                    break;
                }
                match self.scoped(body, env)? {
                    Flow::Break => break,
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                    _ => {}
                }
            },
            StmtKind::Loop(body) => {
                while !self.stopped {
                    self.step(s.span)?;
                    match self.scoped(body, env)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                }
            }
            StmtKind::For {
                name,
                iterable,
                body,
            } => {
                let v = self.eval(iterable, env)?;
                let items: Box<dyn Iterator<Item = Value>> = match v {
                    Value::Range(a, b) => Box::new((a..b).map(Value::Integer)),
                    Value::List(xs) => Box::new(xs.borrow().clone().into_iter()),
                    Value::Text(s) => Box::new(
                        s.chars()
                            .map(|c| Value::Text(c.to_string()))
                            .collect::<Vec<_>>()
                            .into_iter(),
                    ),
                    Value::Map(m) => Box::new(
                        m.borrow()
                            .keys()
                            .cloned()
                            .map(Value::Text)
                            .collect::<Vec<_>>()
                            .into_iter(),
                    ),
                    _ => {
                        return Err(
                            self.error("for requires a list, range, text, or map", iterable.span)
                        )
                    }
                };
                for item in items {
                    if self.stopped {
                        break;
                    }
                    self.step(s.span)?;
                    let local = Environment::new(Some(env.clone()));
                    Environment::define(&local, name, item, false);
                    match self.scoped(body, &local)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                }
            }
            StmtKind::Break => return Ok(Flow::Break),
            StmtKind::Continue => return Ok(Flow::Continue),
            StmtKind::Return(e) => {
                return Ok(Flow::Return(if let Some(e) = e {
                    self.eval(e, env)?
                } else {
                    Value::None
                }))
            }
            StmtKind::Import { path, alias, local } => {
                if !local
                    && path == "text"
                    && alias == "text"
                    && matches!(Environment::get(env, "text"), Some(Value::Type(Type::Text)))
                {
                    return Ok(Flow::Normal);
                }
                let v = if *local {
                    self.import(path, s.span)?
                } else {
                    Value::Namespace(path.clone())
                };
                if Environment::own(env, alias).is_some() {
                    return Err(self.error(format!("'{alias}' is already defined"), s.span));
                }
                Environment::define(env, alias, v, true);
            }
            StmtKind::Every { event, body } => {
                let interval = if let Event::Timer { seconds } = event {
                    let v = self.eval(seconds, env)?;
                    let n = self.number(&v, seconds.span)?;
                    if !(0.000_001..=86400.0 * 365.0).contains(&n) {
                        return Err(self.error(
                            "Timer interval must be between 0.000001 seconds and one year",
                            seconds.span,
                        ));
                    }
                    n
                } else {
                    0.0
                };
                self.handlers.push(Handler {
                    event: event.clone(),
                    body: Rc::new(body.clone()),
                    env: env.clone(),
                    file: self.file.clone(),
                    interval,
                    next: interval,
                });
            }
        }
        Ok(Flow::Normal)
    }
    pub fn eval(&mut self, e: &Expr, env: &Env) -> Result<Value> {
        self.step(e.span)?;
        if self.eval_depth >= 64 {
            return Err(self.error("Maximum expression evaluation depth reached (64)", e.span));
        }
        self.eval_depth += 1;
        let result = self.eval_inner(e, env);
        self.eval_depth -= 1;
        result
    }
    fn eval_inner(&mut self, e: &Expr, env: &Env) -> Result<Value> {
        let value = match &e.kind {
            ExprKind::Literal(l) => match l {
                Literal::Integer(n) => Value::Integer(*n),
                Literal::Decimal(n) => Value::Decimal(*n),
                Literal::Text(s) => Value::Text(s.clone()),
                Literal::Boolean(b) => Value::Boolean(*b),
                Literal::None => Value::None,
            },
            ExprKind::Name(n) => Environment::get(env, n)
                .ok_or_else(|| self.error(format!("Unknown variable '{n}'"), e.span))?,
            ExprKind::List(xs) => {
                let mut values = Vec::new();
                for x in xs {
                    values.push(self.eval(x, env)?);
                }
                Value::list(values)
            }
            ExprKind::Map(xs) => {
                let mut values = BTreeMap::new();
                for (k, x) in xs {
                    values.insert(k.clone(), self.eval(x, env)?);
                }
                Value::map(values)
            }
            ExprKind::Unary(op, x) => {
                let v = self.eval(x, env)?;
                match (op, v) {
                    (Unary::Not, v) => Value::Boolean(!self.boolean(v, e.span)?),
                    (Unary::Neg, Value::Integer(n)) => Value::Integer(
                        n.checked_neg()
                            .ok_or_else(|| self.error("Integer overflow", e.span))?,
                    ),
                    (Unary::Neg, Value::Decimal(n)) => Value::Decimal(-n),
                    (Unary::Pos, v @ Value::Integer(_)) | (Unary::Pos, v @ Value::Decimal(_)) => v,
                    _ => return Err(self.error("Unary +/- requires a number", e.span)),
                }
            }
            ExprKind::Binary(a, Binary::And, b) => {
                let v = self.eval(a, env)?;
                if !self.boolean(v, a.span)? {
                    Value::Boolean(false)
                } else {
                    let v = self.eval(b, env)?;
                    Value::Boolean(self.boolean(v, b.span)?)
                }
            }
            ExprKind::Binary(a, Binary::Or, b) => {
                let v = self.eval(a, env)?;
                if self.boolean(v, a.span)? {
                    Value::Boolean(true)
                } else {
                    let v = self.eval(b, env)?;
                    Value::Boolean(self.boolean(v, b.span)?)
                }
            }
            ExprKind::Binary(a, op, b) => {
                let a = self.eval(a, env)?;
                let b = self.eval(b, env)?;
                self.binary(a, *op, b, e.span)?
            }
            ExprKind::Call(f, args) => {
                let f = self.eval(f, env)?;
                let mut values = Vec::new();
                for a in args {
                    values.push(self.eval(a, env)?);
                }
                self.call(f, values, e.span)?
            }
            ExprKind::Member(x, name) => {
                let x = self.eval(x, env)?;
                self.member(x, name, e.span)?
            }
            ExprKind::Index(x, i) => {
                let x = self.eval(x, env)?;
                let i = self.eval(i, env)?;
                self.index(x, i, e.span)?
            }
        };
        Ok(value)
    }
    pub fn call(&mut self, f: Value, args: Vec<Value>, span: Span) -> Result<Value> {
        match f {
            Value::Builtin(n) | Value::Namespace(n) => stdlib::call(self, &n, args, span),
            Value::Type(t) => stdlib::convert(self, &t, args, span),
            Value::State(success) => {
                if args.len() != 1 {
                    return Err(self.error("success/error expects one argument", span));
                }
                Ok(Value::outcome(success, args[0].clone()))
            }
            Value::Function(f) => {
                if args.len() != f.params.len() {
                    return Err(self.error(
                        format!(
                            "'{}' expects {} arguments, got {}",
                            f.name,
                            f.params.len(),
                            args.len()
                        ),
                        span,
                    ));
                }
                if self.call_depth >= 32 {
                    return Err(self.error("Maximum function call depth reached (32)", span));
                }
                let local = Environment::new(Some(f.closure.clone()));
                for (p, v) in f.params.iter().zip(args) {
                    let v = if let Some(t) = &p.ty {
                        coerce(v, t, span)?
                    } else {
                        v
                    };
                    Environment::define(&local, &p.name, v, false);
                    if let Some(t) = &p.ty {
                        local.borrow_mut().values.get_mut(&p.name).unwrap().ty = t.clone();
                    }
                }
                let previous = std::mem::replace(&mut self.file, f.file.clone());
                self.call_depth += 1;
                let flow = self.execute(&f.body, &local);
                self.call_depth -= 1;
                self.file = previous;
                let value = match flow.map_err(|e| {
                    e.note(format!(
                        "called '{}' at {}:{}:{}",
                        f.name, self.file, span.line, span.column
                    ))
                })? {
                    Flow::Return(v) => v,
                    Flow::Normal => Value::None,
                    _ => return Err(self.error("Invalid control flow escaped a function", span)),
                };
                if let Some(t) = &f.result {
                    coerce(value, t, span).map_err(|e| e.in_file(&self.file))
                } else {
                    Ok(value)
                }
            }
            _ => Err(self.error(format!("A {} value is not callable", f.type_name()), span)),
        }
    }
    pub fn member(&mut self, x: Value, name: &str, span: Span) -> Result<Value> {
        match x {
            Value::Namespace(n) => stdlib::member(self, &n, name, span),
            Value::Type(Type::Text) => stdlib::member(self, "text", name, span),
            Value::Module(env) => Environment::own(&env, name)
                .filter(|_| !name.starts_with('_'))
                .ok_or_else(|| self.error(format!("Module does not export '{name}'"), span)),
            Value::Map(m) => m
                .borrow()
                .get(name)
                .cloned()
                .ok_or_else(|| self.error(format!("Unknown map field '{name}'"), span)),
            Value::List(xs) if name == "length" => Ok(Value::Integer(xs.borrow().len() as i64)),
            Value::Text(s) if name == "length" => Ok(Value::Integer(s.chars().count() as i64)),
            Value::Outcome { success, value } => match name {
                "value" if success => Ok(*value),
                "error" if !success => Ok(*value),
                "ok" => Ok(Value::Boolean(success)),
                "value" | "error" => Err(self.error(
                    "Result has the other state; check it with 'is success' or 'is error' first",
                    span,
                )),
                _ => Err(self.error(format!("Unknown result field '{name}'"), span)),
            },
            _ => Err(self.error(format!("{} has no field '{name}'", x.type_name()), span)),
        }
    }
    pub fn number(&self, v: &Value, span: Span) -> Result<f64> {
        v.number()
            .ok_or_else(|| self.error(format!("Expected a number, got {}", v.type_name()), span))
    }
    pub fn boolean(&self, v: Value, span: Span) -> Result<bool> {
        if let Value::Boolean(b) = v {
            Ok(b)
        } else {
            Err(self.error(format!("Expected boolean, got {}", v.type_name()), span))
        }
    }
    pub fn index(&self, x: Value, i: Value, span: Span) -> Result<Value> {
        match (x, i) {
            (Value::List(xs), Value::Integer(i)) => {
                let xs = xs.borrow();
                let n = normalize_index(i, xs.len())
                    .ok_or_else(|| self.error("List index out of bounds", span))?;
                Ok(xs[n].clone())
            }
            (Value::Text(s), Value::Integer(i)) => {
                let n = normalize_index(i, s.chars().count())
                    .ok_or_else(|| self.error("Text index out of bounds", span))?;
                Ok(Value::Text(s.chars().nth(n).unwrap().to_string()))
            }
            (Value::Map(m), Value::Text(i)) => m
                .borrow()
                .get(&i)
                .cloned()
                .ok_or_else(|| self.error(format!("No map key '{i}'"), span)),
            _ => Err(self.error(
                "Indexing requires list/text with an integer, or map with text",
                span,
            )),
        }
    }
    pub fn binary(&self, a: Value, op: Binary, b: Value, span: Span) -> Result<Value> {
        let fail = || {
            self.error(
                format!(
                    "Cannot apply {op:?} to {} and {}",
                    a.type_name(),
                    b.type_name()
                ),
                span,
            )
        };
        match op {
            Binary::Eq => return Ok(Value::Boolean(a.equals(&b))),
            Binary::Ne => return Ok(Value::Boolean(!a.equals(&b))),
            Binary::Is => {
                return Ok(Value::Boolean(match b {
                    Value::Type(t) => t == Type::Any || a.ty() == t,
                    Value::State(state) => {
                        if let Value::Outcome { success, .. } = a {
                            success == state
                        } else {
                            return Err(self.error("State checks need a result value", span));
                        }
                    }
                    Value::None => matches!(a, Value::None),
                    _ => {
                        return Err(self.error(
                            "Use 'is' for a type or result state; use '==' for value equality",
                            span,
                        ))
                    }
                }))
            }
            Binary::Range => {
                if let (Value::Integer(x), Value::Integer(y)) = (&a, &b) {
                    return Ok(Value::Range(*x, *y));
                } else {
                    return Err(fail());
                }
            }
            Binary::Add => match (&a, &b) {
                (Value::Text(x), Value::Text(y)) => return Ok(Value::Text(format!("{x}{y}"))),
                (Value::List(x), Value::List(y)) => {
                    let mut v = x.borrow().clone();
                    v.extend(y.borrow().iter().cloned());
                    return Ok(Value::list(v));
                }
                _ => {}
            },
            _ => {}
        }
        if let (Value::Text(x), Value::Text(y)) = (&a, &b) {
            return Ok(Value::Boolean(match op {
                Binary::Lt => x < y,
                Binary::Le => x <= y,
                Binary::Gt => x > y,
                Binary::Ge => x >= y,
                _ => return Err(fail()),
            }));
        }
        if let (Value::Integer(x), Value::Integer(y)) = (&a, &b) {
            if matches!(op, Binary::Lt | Binary::Le | Binary::Gt | Binary::Ge) {
                return Ok(Value::Boolean(match op {
                    Binary::Lt => x < y,
                    Binary::Le => x <= y,
                    Binary::Gt => x > y,
                    _ => x >= y,
                }));
            }
            if matches!(
                op,
                Binary::Add | Binary::Sub | Binary::Mul | Binary::Rem | Binary::Pow
            ) {
                if op == Binary::Rem && *y == 0 {
                    return Err(self.error("Remainder by zero", span));
                }
                let n = match op {
                    Binary::Add => x.checked_add(*y),
                    Binary::Sub => x.checked_sub(*y),
                    Binary::Mul => x.checked_mul(*y),
                    Binary::Rem => x.checked_rem(*y),
                    Binary::Pow => {
                        if *y >= 0 && *y <= u32::MAX as i64 {
                            x.checked_pow(*y as u32)
                        } else {
                            return Err(self.error("Integer exponent must be between 0 and 4294967295; use decimal(base) for fractional/negative powers",span));
                        }
                    }
                    _ => unreachable!(),
                };
                return n
                    .map(Value::Integer)
                    .ok_or_else(|| self.error("Integer overflow (signed 64-bit)", span));
            }
        }
        let x = a.number().ok_or_else(fail)?;
        let y = b.number().ok_or_else(fail)?;
        if matches!(op, Binary::Lt | Binary::Le | Binary::Gt | Binary::Ge) {
            return Ok(Value::Boolean(match op {
                Binary::Lt => x < y,
                Binary::Le => x <= y,
                Binary::Gt => x > y,
                _ => x >= y,
            }));
        }
        if matches!(op, Binary::Div | Binary::Rem) && y == 0.0 {
            return Err(self.error("Division by zero", span));
        }
        let n = match op {
            Binary::Add => x + y,
            Binary::Sub => x - y,
            Binary::Mul => x * y,
            Binary::Div => x / y,
            Binary::Rem => x % y,
            Binary::Pow => x.powf(y),
            _ => return Err(fail()),
        };
        if !n.is_finite() {
            return Err(self.error("Decimal operation produced a non-finite value", span));
        }
        Ok(Value::Decimal(n))
    }
    fn import(&mut self, path: &str, span: Span) -> Result<Value> {
        let file = resolve_import(&self.file, path)
            .canonicalize()
            .map_err(|e| self.error(format!("Cannot resolve module '{path}': {e}"), span))?
            .to_string_lossy()
            .into_owned();
        if let Some(e) = self.modules.get(&file) {
            return Ok(Value::Module(e.clone()));
        }
        let body = self
            .program
            .modules
            .get(&file)
            .cloned()
            .ok_or_else(|| self.error("Module was not loaded during checking", span))?;
        let base = Environment::new(None);
        for (n, v) in stdlib::initial_values() {
            Environment::define(&base, &n, v, true);
        }
        Environment::define(&base, "mode", Value::Mode("terminal".into()), false);
        let local = Environment::new(Some(base));
        let previous = std::mem::replace(&mut self.file, file.clone());
        let outcome = self.execute(&body, &local);
        self.file = previous;
        outcome?;
        // Export a view containing only module declarations; functions retain the full lexical environment.
        let exports = local;
        self.modules.insert(file, exports.clone());
        Ok(Value::Module(exports))
    }
    pub fn asset_path(&self, path: &str) -> std::path::PathBuf {
        if Path::new(path).is_absolute() {
            Path::new(path).to_path_buf()
        } else {
            resolve_import(&self.file, path)
        }
    }
    pub fn dispatch(&mut self, index: usize) -> Result<()> {
        let h = self.handlers[index].clone();
        let previous = std::mem::replace(&mut self.file, h.file);
        let result = self.scoped(&h.body, &h.env);
        self.file = previous;
        result.map(|_| ())
    }
    fn location(&mut self, e: &Expr, env: &Env) -> Result<Location> {
        Ok(match &e.kind {
            ExprKind::Name(n) => Location::Name(env.clone(), n.clone()),
            ExprKind::Member(x, n) => Location::Member(self.eval(x, env)?, n.clone()),
            ExprKind::Index(x, i) => {
                let x = self.eval(x, env)?;
                let i = self.eval(i, env)?;
                Location::Index(x, i)
            }
            _ => return Err(self.error("Invalid assignment target", e.span)),
        })
    }
    fn read_location(&mut self, l: &Location, span: Span) -> Result<Value> {
        match l {
            Location::Name(e, n) => Environment::get(e, n)
                .ok_or_else(|| self.error(format!("Unknown variable '{n}'"), span)),
            Location::Member(x, n) => self.member(x.clone(), n, span),
            Location::Index(x, i) => self.index(x.clone(), i.clone(), span),
        }
    }
    fn write_location(
        &mut self,
        l: Location,
        v: Value,
        ty: Option<&Type>,
        span: Span,
    ) -> Result<()> {
        match l {
            Location::Name(e, n) => {
                if n == "mode" {
                    if !matches!(v, Value::Mode(_)) {
                        return Err(
                            self.error("mode must be terminal, background, 2d, or 3d", span)
                        );
                    }
                    if self.frame_count > 0 || self.tick_count > 0 {
                        return Err(self
                            .error("Execution mode cannot change after scheduling starts", span));
                    }
                }
                Environment::assign(&e, &n, v, ty, span)
            }
            Location::Member(Value::Namespace(n), key) => {
                stdlib::set_member(self, &n, &key, v, span)
            }
            Location::Member(Value::Map(m), key)
            | Location::Index(Value::Map(m), Value::Text(key)) => {
                m.borrow_mut().insert(key, v);
                Ok(())
            }
            Location::Index(Value::List(xs), Value::Integer(i)) => {
                let mut xs = xs.borrow_mut();
                let i = normalize_index(i, xs.len())
                    .ok_or_else(|| self.error("List index out of bounds", span))?;
                xs[i] = v;
                Ok(())
            }
            _ => Err(self.error("This value does not support assignment", span)),
        }
        .map_err(|e| e.in_file(&self.file))
    }
}
enum Location {
    Name(Env, String),
    Member(Value, String),
    Index(Value, Value),
}
pub fn normalize_index(i: i64, len: usize) -> Option<usize> {
    let n = if i < 0 {
        (len as i128) + (i as i128)
    } else {
        i as i128
    };
    if n < 0 || n >= len as i128 {
        None
    } else {
        Some(n as usize)
    }
}
