use crate::{
    ast::*,
    diagnostic::{Diagnostic, Result, Span},
    stdlib,
};
use std::collections::BTreeMap;

#[derive(Clone)]
struct Symbol {
    ty: Type,
    readonly: bool,
    arity: Option<usize>,
}
pub fn check(body: &[Stmt]) -> Result<()> {
    let mut c = Checker {
        scopes: vec![BTreeMap::new()],
        loops: 0,
        functions: 0,
        handlers: 0,
        returns: None,
    };
    for (name, value) in stdlib::initial_values() {
        c.scopes[0].insert(
            name,
            Symbol {
                ty: value.ty(),
                readonly: true,
                arity: None,
            },
        );
    }
    c.scopes[0].insert(
        "mode".into(),
        Symbol {
            ty: Type::Any,
            readonly: false,
            arity: None,
        },
    );
    c.block(body, false)
}
struct Checker {
    scopes: Vec<BTreeMap<String, Symbol>>,
    loops: usize,
    functions: usize,
    handlers: usize,
    returns: Option<Type>,
}
impl Checker {
    fn get(&self, n: &str) -> Option<&Symbol> {
        self.scopes.iter().rev().find_map(|s| s.get(n))
    }
    fn block(&mut self, body: &[Stmt], scope: bool) -> Result<()> {
        if scope {
            self.scopes.push(BTreeMap::new());
        }
        let mut names = std::collections::BTreeSet::new();
        for s in body {
            if let StmtKind::Function { name, params, .. } = &s.kind {
                if !names.insert(name.clone()) || self.scopes.last().unwrap().contains_key(name) {
                    return Err(Diagnostic::new(
                        format!("Duplicate or reserved function name '{name}'"),
                        s.span,
                    ));
                }
                self.scopes.last_mut().unwrap().insert(
                    name.clone(),
                    Symbol {
                        ty: Type::Function,
                        readonly: true,
                        arity: Some(params.len()),
                    },
                );
            }
        }
        for s in body {
            if !matches!(s.kind, StmtKind::Function { .. }) {
                self.statement(s)?;
            }
        }
        for s in body {
            if matches!(s.kind, StmtKind::Function { .. }) {
                self.statement(s)?;
            }
        }
        if scope {
            self.scopes.pop();
        }
        Ok(())
    }
    fn compatible(&self, expected: &Type, got: &Type, span: Span) -> Result<()> {
        if expected == got
            || *expected == Type::Any
            || *got == Type::Any
            || (*expected == Type::Decimal && *got == Type::Integer)
        {
            Ok(())
        } else {
            Err(Diagnostic::new(
                format!("Expected {}, got {}", expected.name(), got.name()),
                span,
            ))
        }
    }
    fn condition(&mut self, e: &Expr) -> Result<()> {
        let t = self.expr(e)?;
        self.compatible(&Type::Boolean, &t, e.span)
    }
    fn statement(&mut self, s: &Stmt) -> Result<()> {
        match &s.kind {
            StmtKind::Assign {
                target,
                ty,
                op,
                value,
            } => {
                let mut got = self.expr(value)?;
                if let Some(op) = op {
                    let old = self.expr(target)?;
                    got = self.binary_type(&old, *op, &got, s.span)?;
                }
                if let ExprKind::Name(n) = &target.kind {
                    if let Some(old) = self.get(n) {
                        if old.readonly {
                            return Err(Diagnostic::new(
                                format!("'{n}' is read-only"),
                                target.span,
                            ));
                        }
                        if let Some(t) = ty {
                            if t != &old.ty {
                                return Err(Diagnostic::new(
                                    format!("Cannot change the type of '{n}'"),
                                    target.span,
                                ));
                            }
                        }
                        self.compatible(&old.ty, &got, value.span)?;
                    } else {
                        let declared = ty.clone().unwrap_or(got.clone());
                        self.compatible(&declared, &got, value.span)?;
                        self.scopes.last_mut().unwrap().insert(
                            n.clone(),
                            Symbol {
                                ty: declared,
                                readonly: false,
                                arity: None,
                            },
                        );
                    }
                } else {
                    self.expr(target)?;
                }
            }
            StmtKind::Expression(e) => {
                self.expr(e)?;
            }
            StmtKind::Function {
                params,
                result,
                body,
                ..
            } => {
                let old_loops = self.loops;
                self.loops = 0;
                self.functions += 1;
                let old_returns = self.returns.clone();
                self.returns = result.clone();
                self.scopes.push(BTreeMap::new());
                for p in params {
                    self.scopes.last_mut().unwrap().insert(
                        p.name.clone(),
                        Symbol {
                            ty: p.ty.clone().unwrap_or(Type::Any),
                            readonly: false,
                            arity: None,
                        },
                    );
                }
                // Function bodies are checked after their enclosing declarations.
                self.block(body, false)?;
                self.scopes.pop();
                self.functions -= 1;
                self.loops = old_loops;
                self.returns = old_returns;
            }
            StmtKind::If {
                condition,
                then_body,
                else_body,
            } => {
                self.condition(condition)?;
                self.block(then_body, true)?;
                self.block(else_body, true)?;
            }
            StmtKind::While { condition, body } => {
                self.condition(condition)?;
                self.loops += 1;
                self.block(body, true)?;
                self.loops -= 1;
            }
            StmtKind::For {
                name,
                iterable,
                body,
            } => {
                let ty = self.expr(iterable)?;
                if !matches!(ty, Type::List | Type::Text | Type::Map | Type::Any) {
                    return Err(Diagnostic::new(
                        "for needs a list, range, text, or map",
                        iterable.span,
                    ));
                }
                self.scopes.push(BTreeMap::new());
                let ty = if matches!(&iterable.kind, ExprKind::Binary(_, Binary::Range, _)) {
                    Type::Integer
                } else {
                    Type::Any
                };
                self.scopes.last_mut().unwrap().insert(
                    name.clone(),
                    Symbol {
                        ty,
                        readonly: false,
                        arity: None,
                    },
                );
                self.loops += 1;
                self.block(body, true)?;
                self.loops -= 1;
                self.scopes.pop();
            }
            StmtKind::Loop(body) => {
                self.loops += 1;
                self.block(body, true)?;
                self.loops -= 1;
            }
            StmtKind::Break | StmtKind::Continue => {
                if self.loops == 0 {
                    return Err(Diagnostic::new(
                        "break/continue can only be used inside a loop",
                        s.span,
                    ));
                }
            }
            StmtKind::Return(e) => {
                if self.functions == 0 {
                    return Err(Diagnostic::new(
                        "return can only be used inside a function",
                        s.span,
                    ));
                }
                let got = if let Some(e) = e {
                    self.expr(e)?
                } else {
                    Type::None
                };
                if let Some(expected) = self.returns.clone() {
                    self.compatible(&expected, &got, s.span)?;
                }
            }
            StmtKind::Import { path, alias, local } => {
                if !local && !stdlib::has_module(path) {
                    return Err(Diagnostic::new(
                        format!("Unknown built-in module '{path}'"),
                        s.span,
                    ));
                }
                if self.scopes.last().unwrap().contains_key(alias) {
                    if !local && path == "text" && alias == "text" {
                        return Ok(());
                    }
                    return Err(Diagnostic::new(
                        format!("'{alias}' is already defined"),
                        s.span,
                    ));
                }
                self.scopes.last_mut().unwrap().insert(
                    alias.clone(),
                    Symbol {
                        ty: Type::Any,
                        readonly: true,
                        arity: None,
                    },
                );
            }
            StmtKind::Every { event, body } => {
                if self.functions > 0
                    || self.loops > 0
                    || self.handlers > 0
                    || self.scopes.len() != 1
                {
                    return Err(Diagnostic::new(
                        "Recurring handlers must be declared at module level",
                        s.span,
                    ));
                }
                if let Event::Timer { seconds } = event {
                    let t = self.expr(seconds)?;
                    if !matches!(t, Type::Integer | Type::Decimal | Type::Any) {
                        return Err(Diagnostic::new(
                            "Timer interval must be a number",
                            seconds.span,
                        ));
                    }
                }
                let loops = self.loops;
                self.loops = 0;
                self.handlers += 1;
                self.block(body, true)?;
                self.handlers -= 1;
                self.loops = loops;
            }
        }
        Ok(())
    }
    fn expr(&mut self, e: &Expr) -> Result<Type> {
        Ok(match &e.kind {
            ExprKind::Literal(l) => match l {
                Literal::Integer(_) => Type::Integer,
                Literal::Decimal(_) => Type::Decimal,
                Literal::Text(_) => Type::Text,
                Literal::Boolean(_) => Type::Boolean,
                Literal::None => Type::None,
            },
            ExprKind::Name(n) => {
                if let Some(s) = self.get(n) {
                    s.ty.clone()
                } else {
                    return Err(Diagnostic::new(format!("Unknown variable '{n}'"), e.span));
                }
            }
            ExprKind::List(xs) => {
                for x in xs {
                    self.expr(x)?;
                }
                Type::List
            }
            ExprKind::Map(xs) => {
                for (_, x) in xs {
                    self.expr(x)?;
                }
                Type::Map
            }
            ExprKind::Unary(op, x) => {
                let t = self.expr(x)?;
                match op {
                    Unary::Not => {
                        self.compatible(&Type::Boolean, &t, x.span)?;
                        Type::Boolean
                    }
                    _ => {
                        if !matches!(t, Type::Integer | Type::Decimal | Type::Any) {
                            return Err(Diagnostic::new("Unary +/- requires a number", e.span));
                        }
                        t
                    }
                }
            }
            ExprKind::Binary(a, op, b) => {
                let ta = self.expr(a)?;
                let tb = self.expr(b)?;
                self.binary_type(&ta, *op, &tb, e.span)?
            }
            ExprKind::Member(x, _) => {
                self.expr(x)?;
                Type::Any
            }
            ExprKind::Index(x, i) => {
                self.expr(x)?;
                self.expr(i)?;
                Type::Any
            }
            ExprKind::Call(f, args) => {
                let t = self.expr(f)?;
                if let ExprKind::Name(n) = &f.kind {
                    if let Some(arity) = self.get(n).and_then(|s| s.arity) {
                        if args.len() != arity {
                            return Err(Diagnostic::new(
                                format!("'{n}' expects {arity} arguments, got {}", args.len()),
                                e.span,
                            ));
                        }
                    }
                }
                if !matches!(t, Type::Function | Type::Any) {
                    return Err(Diagnostic::new(
                        format!("A {} value is not callable", t.name()),
                        f.span,
                    ));
                }
                for a in args {
                    self.expr(a)?;
                }
                Type::Any
            }
        })
    }
    fn binary_type(&self, a: &Type, op: Binary, b: &Type, span: Span) -> Result<Type> {
        match op {
            Binary::Eq | Binary::Ne | Binary::Is => Ok(Type::Boolean),
            Binary::And | Binary::Or => {
                self.compatible(&Type::Boolean, a, span)?;
                self.compatible(&Type::Boolean, b, span)?;
                Ok(Type::Boolean)
            }
            Binary::Range => {
                self.compatible(&Type::Integer, a, span)?;
                self.compatible(&Type::Integer, b, span)?;
                Ok(Type::List)
            }
            Binary::Add if *a == Type::Text && *b == Type::Text => Ok(Type::Text),
            Binary::Add if *a == Type::List && *b == Type::List => Ok(Type::List),
            Binary::Lt | Binary::Le | Binary::Gt | Binary::Ge
                if *a == Type::Text && *b == Type::Text =>
            {
                Ok(Type::Boolean)
            }
            _ => {
                if *a == Type::Any || *b == Type::Any {
                    return Ok(
                        if matches!(op, Binary::Lt | Binary::Le | Binary::Gt | Binary::Ge) {
                            Type::Boolean
                        } else {
                            Type::Any
                        },
                    );
                }
                if !matches!(a, Type::Integer | Type::Decimal)
                    || !matches!(b, Type::Integer | Type::Decimal)
                {
                    return Err(Diagnostic::new(
                        format!("Cannot apply {op:?} to {} and {}", a.name(), b.name()),
                        span,
                    ));
                }
                Ok(
                    if matches!(op, Binary::Lt | Binary::Le | Binary::Gt | Binary::Ge) {
                        Type::Boolean
                    } else if op == Binary::Div || *a == Type::Decimal || *b == Type::Decimal {
                        Type::Decimal
                    } else {
                        Type::Integer
                    },
                )
            }
        }
    }
}
