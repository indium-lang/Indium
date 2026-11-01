use crate::diagnostic::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Decimal(f64),
    Text(String),
    Boolean(bool),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binary {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Is,
    Range,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unary {
    Neg,
    Not,
    Pos,
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum ExprKind {
    Literal(Literal),
    Name(String),
    List(Vec<Expr>),
    Map(Vec<(String, Expr)>),
    Unary(Unary, Box<Expr>),
    Binary(Box<Expr>, Binary, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Member(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
}
impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Integer,
    Decimal,
    Text,
    Boolean,
    List,
    Map,
    Function,
    None,
    Any,
}
impl Type {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "integer" => Self::Integer,
            "decimal" => Self::Decimal,
            "text" => Self::Text,
            "boolean" => Self::Boolean,
            "list" => Self::List,
            "map" => Self::Map,
            "function" => Self::Function,
            "none" => Self::None,
            "any" => Self::Any,
            _ => return None,
        })
    }
    pub fn name(&self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::Decimal => "decimal",
            Self::Text => "text",
            Self::Boolean => "boolean",
            Self::List => "list",
            Self::Map => "map",
            Self::Function => "function",
            Self::None => "none",
            Self::Any => "any",
        }
    }
}
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub ty: Option<Type>,
}
#[derive(Debug, Clone)]
pub enum Event {
    Tick,
    Frame,
    Timer { seconds: Expr },
}
#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}
#[derive(Debug, Clone)]
pub enum StmtKind {
    Assign {
        target: Expr,
        ty: Option<Type>,
        op: Option<Binary>,
        value: Expr,
    },
    Expression(Expr),
    Function {
        name: String,
        params: Vec<Parameter>,
        result: Option<Type>,
        body: Vec<Stmt>,
    },
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    For {
        name: String,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    Loop(Vec<Stmt>),
    Break,
    Continue,
    Return(Option<Expr>),
    Import {
        path: String,
        alias: String,
        local: bool,
    },
    Every {
        event: Event,
        body: Vec<Stmt>,
    },
}
