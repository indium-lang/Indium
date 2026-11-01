use super::values::Value;
use crate::{
    ast::Type,
    diagnostic::{Diagnostic, Result, Span},
};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
pub type Env = Rc<RefCell<Environment>>;
#[derive(Clone)]
pub struct Binding {
    pub value: Value,
    pub ty: Type,
    pub readonly: bool,
}
pub struct Environment {
    pub values: BTreeMap<String, Binding>,
    pub parent: Option<Env>,
}
impl Environment {
    pub fn new(parent: Option<Env>) -> Env {
        Rc::new(RefCell::new(Self {
            values: BTreeMap::new(),
            parent,
        }))
    }
    pub fn define(env: &Env, name: &str, value: Value, readonly: bool) {
        let ty = value.ty();
        env.borrow_mut().values.insert(
            name.into(),
            Binding {
                value,
                ty,
                readonly,
            },
        );
    }
    pub fn get(env: &Env, name: &str) -> Option<Value> {
        let e = env.borrow();
        e.values
            .get(name)
            .map(|b| b.value.clone())
            .or_else(|| e.parent.as_ref().and_then(|p| Self::get(p, name)))
    }
    pub fn own(env: &Env, name: &str) -> Option<Value> {
        env.borrow().values.get(name).map(|b| b.value.clone())
    }
    pub fn assign(
        env: &Env,
        name: &str,
        value: Value,
        annotation: Option<&Type>,
        span: Span,
    ) -> Result<()> {
        if env.borrow().values.contains_key(name) {
            let mut e = env.borrow_mut();
            let b = e.values.get_mut(name).expect("binding exists");
            if b.readonly {
                return Err(Diagnostic::new(format!("'{name}' is read-only"), span));
            }
            if let Some(t) = annotation {
                if t != &b.ty {
                    return Err(Diagnostic::new(
                        format!("Cannot change the declared type of '{name}'"),
                        span,
                    ));
                }
            }
            b.value = coerce(value, &b.ty, span)?;
            return Ok(());
        }
        let parent = env.borrow().parent.clone();
        if let Some(p) = parent {
            if Self::get(&p, name).is_some() {
                return Self::assign(&p, name, value, annotation, span);
            }
        }
        let ty = annotation.cloned().unwrap_or_else(|| value.ty());
        let value = coerce(value, &ty, span)?;
        env.borrow_mut().values.insert(
            name.into(),
            Binding {
                value,
                ty,
                readonly: false,
            },
        );
        Ok(())
    }
}
pub fn coerce(value: Value, ty: &Type, span: Span) -> Result<Value> {
    if matches!(ty, Type::Any) || value.ty() == *ty {
        Ok(value)
    } else if *ty == Type::Decimal {
        if let Value::Integer(n) = value {
            Ok(Value::Decimal(n as f64))
        } else {
            Err(Diagnostic::new(
                format!("Expected decimal, got {}", value.type_name()),
                span,
            ))
        }
    } else {
        Err(Diagnostic::new(
            format!("Expected {}, got {}", ty.name(), value.type_name()),
            span,
        ))
    }
}
