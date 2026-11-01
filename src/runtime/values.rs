use super::environment::Env;
use crate::ast::{Parameter, Stmt, Type};
use std::{cell::RefCell, collections::BTreeMap, fmt, rc::Rc};

#[derive(Clone)]
pub enum Value {
    Integer(i64),
    Decimal(f64),
    Text(String),
    Boolean(bool),
    None,
    List(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<BTreeMap<String, Value>>>),
    Function(Rc<Function>),
    Builtin(String),
    Namespace(String),
    Module(Env),
    Type(Type),
    State(bool),
    Mode(String),
    Range(i64, i64),
    Outcome { success: bool, value: Box<Value> },
}
#[derive(Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Parameter>,
    pub result: Option<Type>,
    pub body: Vec<Stmt>,
    pub closure: Env,
    pub file: String,
}
impl Value {
    pub fn list(items: Vec<Value>) -> Self {
        Self::List(Rc::new(RefCell::new(items)))
    }
    pub fn map(items: BTreeMap<String, Value>) -> Self {
        Self::Map(Rc::new(RefCell::new(items)))
    }
    pub fn outcome(success: bool, value: Value) -> Self {
        Self::Outcome {
            success,
            value: Box::new(value),
        }
    }
    pub fn ty(&self) -> Type {
        match self {
            Self::Integer(_) => Type::Integer,
            Self::Decimal(_) => Type::Decimal,
            Self::Text(_) => Type::Text,
            Self::Boolean(_) => Type::Boolean,
            Self::None => Type::None,
            Self::List(_) | Self::Range(_, _) => Type::List,
            Self::Map(_) | Self::Module(_) => Type::Map,
            Self::Function(_) | Self::Builtin(_) => Type::Function,
            _ => Type::Any,
        }
    }
    pub fn type_name(&self) -> &str {
        match self {
            Self::Outcome { .. } => "result",
            Self::Namespace(_) => "module",
            Self::Type(_) => "type",
            Self::State(_) => "state",
            Self::Mode(_) => "mode",
            _ => self.ty().name(),
        }
    }
    pub fn number(&self) -> Option<f64> {
        match self {
            Self::Integer(x) => Some(*x as f64),
            Self::Decimal(x) => Some(*x),
            _ => None,
        }
    }
    pub fn equals(&self, other: &Self) -> bool {
        self.equal_depth(other, 0)
    }
    fn equal_depth(&self, other: &Self, depth: usize) -> bool {
        if depth > 64 {
            return false;
        }
        match (self, other) {
            (Self::Integer(a), Self::Integer(b)) => a == b,
            (Self::Decimal(a), Self::Decimal(b)) => a == b,
            (Self::Integer(a), Self::Decimal(b)) | (Self::Decimal(b), Self::Integer(a)) => {
                *b >= i64::MIN as f64
                    && *b < (i64::MAX as f64)
                    && b.fract() == 0.0
                    && *a == *b as i64
            }
            (Self::Text(a), Self::Text(b)) => a == b,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::None, Self::None) => true,
            (Self::Type(a), Self::Type(b)) => a == b,
            (Self::State(a), Self::State(b)) => a == b,
            (Self::Mode(a), Self::Mode(b)) => a == b,
            (Self::Range(a, b), Self::Range(c, d)) => a == c && b == d,
            (Self::List(a), Self::List(b)) => {
                Rc::ptr_eq(a, b) || {
                    let a = a.borrow();
                    let b = b.borrow();
                    a.len() == b.len()
                        && a.iter()
                            .zip(b.iter())
                            .all(|(a, b)| a.equal_depth(b, depth + 1))
                }
            }
            (Self::Map(a), Self::Map(b)) => {
                Rc::ptr_eq(a, b) || {
                    let a = a.borrow();
                    let b = b.borrow();
                    a.len() == b.len()
                        && a.iter()
                            .all(|(k, a)| b.get(k).is_some_and(|b| a.equal_depth(b, depth + 1)))
                }
            }
            (
                Self::Outcome {
                    success: a,
                    value: x,
                },
                Self::Outcome {
                    success: b,
                    value: y,
                },
            ) => a == b && x.equal_depth(y, depth + 1),
            (Self::Function(a), Self::Function(b)) => Rc::ptr_eq(a, b),
            (Self::Builtin(a), Self::Builtin(b)) => a == b,
            _ => false,
        }
    }
    fn display_depth(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
        if depth > 8 {
            return write!(f, "...");
        }
        match self {
            Self::Integer(x) => write!(f, "{x}"),
            Self::Decimal(x) => write!(f, "{x}"),
            Self::Text(x) => write!(f, "{x}"),
            Self::Boolean(x) => write!(f, "{x}"),
            Self::None => write!(f, "none"),
            Self::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    item.display_depth(f, depth + 1)?;
                }
                write!(f, "]")
            }
            Self::Map(items) => {
                write!(f, "{{")?;
                for (i, (k, item)) in items.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: ")?;
                    item.display_depth(f, depth + 1)?;
                }
                write!(f, "}}")
            }
            Self::Function(x) => write!(f, "<fn {}>", x.name),
            Self::Builtin(x) => write!(f, "<builtin {x}>"),
            Self::Namespace(x) => write!(f, "<module {x}>"),
            Self::Module(_) => write!(f, "<module>"),
            Self::Type(x) => write!(f, "<type {}>", x.name()),
            Self::State(x) => write!(f, "{}", if *x { "success" } else { "error" }),
            Self::Mode(x) => write!(f, "{x}"),
            Self::Range(a, b) => write!(f, "{a}..{b}"),
            Self::Outcome { success, value } => {
                write!(f, "{}(", if *success { "success" } else { "error" })?;
                value.display_depth(f, depth + 1)?;
                write!(f, ")")
            }
        }
    }
}
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.display_depth(f, 0)
    }
}
impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
