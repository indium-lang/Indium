use crate::ast::*;
use crate::diagnostic::{Diagnostic, Result, Span};
use crate::lexer::{self, Kind, Token};

pub fn parse(source: &str) -> Result<Vec<Stmt>> {
    Parser {
        tokens: lexer::lex(source)?,
        pos: 0,
        depth: 0,
    }
    .program(false)
}
struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
}
impl Parser {
    fn peek(&self) -> &Kind {
        &self.tokens[self.pos].kind
    }
    fn span(&self) -> Span {
        self.tokens[self.pos].span
    }
    fn bump(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if !matches!(t.kind, Kind::Eof) {
            self.pos += 1;
        }
        t
    }
    fn at(&self, k: &Kind) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(k)
    }
    fn eat(&mut self, k: &Kind) -> bool {
        if self.at(k) {
            self.bump();
            true
        } else {
            false
        }
    }
    fn word(&self, s: &str) -> bool {
        matches!(self.peek(),Kind::Ident(w) if w==s)
    }
    fn eat_word(&mut self, s: &str) -> bool {
        if self.word(s) {
            self.bump();
            true
        } else {
            false
        }
    }
    fn expect(&mut self, k: &Kind, desc: &str) -> Result<()> {
        if self.eat(k) {
            Ok(())
        } else {
            Err(self.err(format!("Expected {desc}, found {}", self.describe())))
        }
    }
    fn err(&self, m: impl Into<String>) -> Diagnostic {
        Diagnostic::new(m, self.span())
    }
    fn describe(&self) -> String {
        match self.peek() {
            Kind::Eof => "end of file".into(),
            Kind::Newline => "a newline".into(),
            Kind::Ident(s) => format!("'{s}'"),
            k => format!("{k:?}"),
        }
    }
    fn name(&mut self) -> Result<String> {
        match self.bump() {
            Token {
                kind: Kind::Ident(s),
                span,
            } => {
                if is_reserved(&s) {
                    Err(Diagnostic::new(format!("'{s}' is reserved"), span))
                } else {
                    Ok(s)
                }
            }
            t => Err(Diagnostic::new("Expected a name", t.span)),
        }
    }
    fn newlines(&mut self) {
        while self.eat(&Kind::Newline) {}
    }
    fn separators(&mut self) {
        while self.eat(&Kind::Newline) || self.eat(&Kind::Semicolon) {}
    }
    fn program(&mut self, in_block: bool) -> Result<Vec<Stmt>> {
        let mut body = Vec::new();
        self.separators();
        while !self.at(&Kind::Eof) && !(in_block && self.at(&Kind::RBrace)) {
            body.push(self.statement()?);
            if !self.at(&Kind::Eof)
                && !self.at(&Kind::RBrace)
                && !self.at(&Kind::Newline)
                && !self.at(&Kind::Semicolon)
            {
                return Err(self.err("Expected a newline or ';' between statements"));
            }
            self.separators();
        }
        Ok(body)
    }
    fn block(&mut self) -> Result<Vec<Stmt>> {
        self.newlines();
        self.expect(&Kind::LBrace, "'{'")?;
        self.depth += 1;
        if self.depth > 128 {
            return Err(self.err("Blocks are nested too deeply (limit: 128)"));
        }
        let body = self.program(true)?;
        self.expect(&Kind::RBrace, "'}'")?;
        self.depth -= 1;
        Ok(body)
    }
    fn statement(&mut self) -> Result<Stmt> {
        let span = self.span();
        let kind = if self.eat_word("fn") {
            self.function()?
        } else if self.eat_word("if") {
            self.if_statement()?
        } else if self.eat_word("while") {
            let condition = self.expr(0)?;
            let body = self.block()?;
            StmtKind::While { condition, body }
        } else if self.eat_word("for") {
            let name = self.name()?;
            if !self.eat_word("in") {
                return Err(self.err("Expected 'in' after the loop variable"));
            }
            let iterable = self.expr(0)?;
            let body = self.block()?;
            StmtKind::For {
                name,
                iterable,
                body,
            }
        } else if self.eat_word("loop") {
            StmtKind::Loop(self.block()?)
        } else if self.eat_word("break") {
            StmtKind::Break
        } else if self.eat_word("continue") {
            StmtKind::Continue
        } else if self.eat_word("return") {
            let e = if matches!(
                self.peek(),
                Kind::Newline | Kind::Semicolon | Kind::RBrace | Kind::Eof
            ) {
                None
            } else {
                Some(self.expr(0)?)
            };
            StmtKind::Return(e)
        } else if self.eat_word("import") {
            self.import()?
        } else if self.eat_word("every") {
            self.every()?
        } else {
            let target = self.expr(0)?;
            let ty = if self.eat(&Kind::Colon) {
                Some(self.ty()?)
            } else {
                None
            };
            let op = match self.peek() {
                Kind::Eq => Some(None),
                Kind::PlusEq => Some(Some(Binary::Add)),
                Kind::MinusEq => Some(Some(Binary::Sub)),
                Kind::StarEq => Some(Some(Binary::Mul)),
                Kind::SlashEq => Some(Some(Binary::Div)),
                Kind::PercentEq => Some(Some(Binary::Rem)),
                _ => None,
            };
            if let Some(op) = op {
                self.bump();
                if !matches!(
                    target.kind,
                    ExprKind::Name(_) | ExprKind::Member(_, _) | ExprKind::Index(_, _)
                ) {
                    return Err(Diagnostic::new("Invalid assignment target", target.span));
                }
                if ty.is_some() && !matches!(target.kind, ExprKind::Name(_)) {
                    return Err(Diagnostic::new(
                        "Type annotations belong on variable names",
                        target.span,
                    ));
                }
                if ty.is_some() && op.is_some() {
                    return Err(Diagnostic::new(
                        "Use '=' with a type annotation",
                        target.span,
                    ));
                }
                let value = self.expr(0)?;
                StmtKind::Assign {
                    target,
                    ty,
                    op,
                    value,
                }
            } else {
                if ty.is_some() {
                    return Err(self.err("Expected '=' after a type annotation"));
                }
                StmtKind::Expression(target)
            }
        };
        Ok(Stmt { kind, span })
    }
    fn ty(&mut self) -> Result<Type> {
        match self.bump() {
            Token {
                kind: Kind::Ident(s),
                span,
            } => {
                Type::parse(&s).ok_or_else(|| Diagnostic::new(format!("Unknown type '{s}'"), span))
            }
            t => Err(Diagnostic::new("Expected a type name", t.span)),
        }
    }
    fn function(&mut self) -> Result<StmtKind> {
        let name = self.name()?;
        self.expect(&Kind::LParen, "'('")?;
        self.newlines();
        let mut params = Vec::new();
        while !self.at(&Kind::RParen) {
            let n = self.name()?;
            if params.iter().any(|p: &Parameter| p.name == n) {
                return Err(self.err(format!("Duplicate parameter '{n}'")));
            }
            let ty = if self.eat(&Kind::Colon) {
                Some(self.ty()?)
            } else {
                None
            };
            params.push(Parameter { name: n, ty });
            self.newlines();
            if !self.eat(&Kind::Comma) {
                break;
            }
            self.newlines();
        }
        self.expect(&Kind::RParen, "')'")?;
        let result = if self.eat(&Kind::Arrow) {
            Some(self.ty()?)
        } else {
            None
        };
        let body = self.block()?;
        Ok(StmtKind::Function {
            name,
            params,
            result,
            body,
        })
    }
    fn if_statement(&mut self) -> Result<StmtKind> {
        let condition = self.expr(0)?;
        let then_body = self.block()?;
        let saved = self.pos;
        self.newlines();
        let else_body = if self.eat_word("else") {
            if self.eat_word("if") {
                let span = self.span();
                vec![Stmt {
                    kind: self.if_statement()?,
                    span,
                }]
            } else {
                self.block()?
            }
        } else {
            self.pos = saved;
            Vec::new()
        };
        Ok(StmtKind::If {
            condition,
            then_body,
            else_body,
        })
    }
    fn import(&mut self) -> Result<StmtKind> {
        let (path, local) = match self.bump() {
            Token {
                kind: Kind::Ident(s),
                ..
            } => (s, false),
            Token {
                kind: Kind::Text(s),
                ..
            } => (s, true),
            t => {
                return Err(Diagnostic::new(
                    "Expected a built-in module name or a quoted .in path",
                    t.span,
                ))
            }
        };
        let alias = if self.eat_word("as") {
            self.name()?
        } else if local {
            std::path::Path::new(&path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("module")
                .replace('-', "_")
        } else {
            path.clone()
        };
        if alias.is_empty()
            || !alias
                .chars()
                .enumerate()
                .all(|(i, c)| c == '_' || c.is_alphabetic() || (i > 0 && c.is_numeric()))
            || is_reserved(&alias)
        {
            return Err(self.err("Invalid module name; use 'as name' to set an alias"));
        }
        Ok(StmtKind::Import { path, alias, local })
    }
    fn every(&mut self) -> Result<StmtKind> {
        let event = if self.eat_word("tick") {
            Event::Tick
        } else if self.eat_word("frame") {
            Event::Frame
        } else {
            let n = self.expr(0)?;
            let seconds = if self.eat_word("seconds") || self.eat_word("second") {
                n
            } else if self.eat_word("milliseconds") || self.eat_word("millisecond") {
                let span = n.span;
                Expr::new(
                    ExprKind::Binary(
                        Box::new(n),
                        Binary::Div,
                        Box::new(Expr::new(ExprKind::Literal(Literal::Integer(1000)), span)),
                    ),
                    span,
                )
            } else {
                return Err(self.err("Expected tick, frame, seconds, or milliseconds"));
            };
            Event::Timer { seconds }
        };
        let body = self.block()?;
        self.newlines();
        if !self.eat_word("loop") {
            return Err(
                self.err("Recurring handlers must end with 'loop': every tick { ... } loop")
            );
        }
        Ok(StmtKind::Every { event, body })
    }
    fn expr(&mut self, min: u8) -> Result<Expr> {
        self.newlines();
        self.depth += 1;
        if self.depth > 128 {
            return Err(self.err("Expression is nested too deeply (limit: 128)"));
        }
        let mut left = self.prefix()?;
        let mut chain = 0;
        loop {
            chain += 1;
            if chain > 128 {
                return Err(self.err("Expression is nested too deeply (limit: 128)"));
            }
            if min <= 20 {
                if self.eat(&Kind::LParen) {
                    let span = left.span;
                    let mut args = Vec::new();
                    self.newlines();
                    while !self.at(&Kind::RParen) {
                        args.push(self.expr(0)?);
                        self.newlines();
                        if !self.eat(&Kind::Comma) {
                            break;
                        }
                        self.newlines();
                    }
                    self.expect(&Kind::RParen, "')'")?;
                    left = Expr::new(ExprKind::Call(Box::new(left), args), span);
                    continue;
                }
                if self.eat(&Kind::Dot) {
                    let span = left.span;
                    let name = match self.bump() {
                        Token {
                            kind: Kind::Ident(n),
                            ..
                        } => n,
                        t => return Err(Diagnostic::new("Expected a member name", t.span)),
                    };
                    left = Expr::new(ExprKind::Member(Box::new(left), name), span);
                    continue;
                }
                if self.eat(&Kind::LBracket) {
                    let span = left.span;
                    let index = self.expr(0)?;
                    self.newlines();
                    self.expect(&Kind::RBracket, "']'")?;
                    left = Expr::new(ExprKind::Index(Box::new(left), Box::new(index)), span);
                    continue;
                }
            }
            let Some((op, l, r)) = self.binary() else {
                break;
            };
            if l < min {
                break;
            }
            self.bump();
            let right = self.expr(r)?;
            let span = left.span;
            left = Expr::new(ExprKind::Binary(Box::new(left), op, Box::new(right)), span);
        }
        self.depth -= 1;
        Ok(left)
    }
    fn binary(&self) -> Option<(Binary, u8, u8)> {
        Some(match self.peek() {
            Kind::Or => (Binary::Or, 1, 2),
            Kind::And => (Binary::And, 3, 4),
            Kind::EqEq => (Binary::Eq, 5, 6),
            Kind::Ne => (Binary::Ne, 5, 6),
            Kind::Lt => (Binary::Lt, 7, 8),
            Kind::Le => (Binary::Le, 7, 8),
            Kind::Gt => (Binary::Gt, 7, 8),
            Kind::Ge => (Binary::Ge, 7, 8),
            Kind::Ident(s) if s == "is" => (Binary::Is, 7, 8),
            Kind::Range => (Binary::Range, 9, 10),
            Kind::Plus => (Binary::Add, 11, 12),
            Kind::Minus => (Binary::Sub, 11, 12),
            Kind::Star => (Binary::Mul, 13, 14),
            Kind::Slash => (Binary::Div, 13, 14),
            Kind::Percent => (Binary::Rem, 13, 14),
            Kind::Pow => (Binary::Pow, 17, 16),
            _ => return None,
        })
    }
    fn prefix(&mut self) -> Result<Expr> {
        let t = self.bump();
        let span = t.span;
        let kind = match t.kind {
            Kind::Integer(n) => ExprKind::Literal(Literal::Integer(n)),
            Kind::Decimal(n) => ExprKind::Literal(Literal::Decimal(n)),
            Kind::Text(s) => ExprKind::Literal(Literal::Text(s)),
            Kind::Ident(s) => match s.as_str() {
                "true" => ExprKind::Literal(Literal::Boolean(true)),
                "false" => ExprKind::Literal(Literal::Boolean(false)),
                "none" => ExprKind::Literal(Literal::None),
                s if is_control(s) => {
                    return Err(Diagnostic::new(
                        format!("Expected an expression, found '{s}'"),
                        span,
                    ))
                }
                _ => ExprKind::Name(s),
            },
            Kind::Minus => ExprKind::Unary(Unary::Neg, Box::new(self.expr(15)?)),
            Kind::Plus => ExprKind::Unary(Unary::Pos, Box::new(self.expr(15)?)),
            Kind::Not => ExprKind::Unary(Unary::Not, Box::new(self.expr(5)?)),
            Kind::LParen => {
                let e = self.expr(0)?;
                self.newlines();
                if self.eat(&Kind::Comma) {
                    let mut items = vec![e];
                    self.newlines();
                    while !self.at(&Kind::RParen) {
                        items.push(self.expr(0)?);
                        self.newlines();
                        if !self.eat(&Kind::Comma) {
                            break;
                        }
                        self.newlines();
                    }
                    self.expect(&Kind::RParen, "')'")?;
                    ExprKind::List(items)
                } else {
                    self.expect(&Kind::RParen, "')'")?;
                    return Ok(e);
                }
            }
            Kind::LBracket => {
                let mut items = Vec::new();
                self.newlines();
                while !self.at(&Kind::RBracket) {
                    items.push(self.expr(0)?);
                    self.newlines();
                    if !self.eat(&Kind::Comma) {
                        break;
                    }
                    self.newlines();
                }
                self.expect(&Kind::RBracket, "']'")?;
                ExprKind::List(items)
            }
            Kind::LBrace => {
                let mut items = Vec::new();
                self.newlines();
                while !self.at(&Kind::RBrace) {
                    let k = match self.bump() {
                        Token {
                            kind: Kind::Ident(s) | Kind::Text(s),
                            ..
                        } => s,
                        t => return Err(Diagnostic::new("Expected a map key", t.span)),
                    };
                    if items.iter().any(|(key, _)| key == &k) {
                        return Err(self.err(format!("Duplicate map key '{k}'")));
                    }
                    self.expect(&Kind::Colon, "':'")?;
                    let v = self.expr(0)?;
                    items.push((k, v));
                    self.newlines();
                    if !self.eat(&Kind::Comma) {
                        break;
                    }
                    self.newlines();
                }
                self.expect(&Kind::RBrace, "'}'")?;
                ExprKind::Map(items)
            }
            _ => {
                return Err(Diagnostic::new(
                    format!("Expected an expression, found {:?}", t.kind),
                    span,
                ))
            }
        };
        Ok(Expr::new(kind, span))
    }
}
fn is_control(s: &str) -> bool {
    matches!(
        s,
        "fn" | "if"
            | "else"
            | "while"
            | "for"
            | "in"
            | "loop"
            | "break"
            | "continue"
            | "return"
            | "import"
            | "as"
            | "every"
            | "is"
    )
}
fn is_reserved(s: &str) -> bool {
    is_control(s)
        || matches!(
            s,
            "true"
                | "false"
                | "none"
                | "terminal"
                | "background"
                | "2d"
                | "3d"
                | "success"
                | "error"
        )
}
