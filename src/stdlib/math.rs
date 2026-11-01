use super::Args;
use crate::{diagnostic::Result, interpreter::Interpreter, runtime::values::Value};
pub fn call(i: &mut Interpreter, n: &str, a: Args<'_>) -> Result<Value> {
    if n == "seed" {
        a.exact(i, 1)?;
        i.rng = a.int(i, 0)? as u64;
        if i.rng == 0 {
            i.rng = 49;
        }
        return Ok(Value::None);
    }
    if n == "random" || n == "random_int" {
        a.exact(i, if n == "random" { 0 } else { 2 })?;
        i.rng ^= i.rng << 13;
        i.rng ^= i.rng >> 7;
        i.rng ^= i.rng << 17;
        if n == "random" {
            return Ok(Value::Decimal((i.rng >> 11) as f64 / (1u64 << 53) as f64));
        }
        let lo = a.int(i, 0)?;
        let hi = a.int(i, 1)?;
        if lo > hi {
            return Err(i.error("random_int minimum exceeds maximum", a.span));
        }
        let width = hi as i128 - lo as i128 + 1;
        let offset = (i.rng as u128) % (width as u128);
        return Ok(Value::Integer((lo as i128 + offset as i128) as i64));
    }
    a.exact(
        i,
        match n {
            "pow" | "min" | "max" => 2,
            "clamp" => 3,
            _ => 1,
        },
    )?;
    let x = a.number(i, 0)?;
    if n == "abs" {
        if let Value::Integer(n) = a.values[0] {
            return n
                .checked_abs()
                .map(Value::Integer)
                .ok_or_else(|| i.error("Integer overflow", a.span));
        }
    }
    if n == "min" || n == "max" {
        if let (Value::Integer(x), Value::Integer(y)) = (&a.values[0], &a.values[1]) {
            return Ok(Value::Integer(if n == "min" {
                *x.min(y)
            } else {
                *x.max(y)
            }));
        }
    }
    let v = match n {
        "sqrt" => x.sqrt(),
        "abs" => x.abs(),
        "floor" => x.floor(),
        "ceil" => x.ceil(),
        "round" => x.round(),
        "sin" => x.sin(),
        "cos" => x.cos(),
        "tan" => x.tan(),
        "log" => x.ln(),
        "exp" => x.exp(),
        "pow" => x.powf(a.number(i, 1)?),
        "min" => x.min(a.number(i, 1)?),
        "max" => x.max(a.number(i, 1)?),
        "clamp" => {
            let lo = a.number(i, 1)?;
            let hi = a.number(i, 2)?;
            if lo > hi {
                return Err(i.error("clamp minimum exceeds maximum", a.span));
            }
            x.clamp(lo, hi)
        }
        _ => return Err(i.error(format!("Unknown math function '{n}'"), a.span)),
    };
    if !v.is_finite() {
        return Err(i.error("Math operation is outside its finite domain", a.span));
    }
    Ok(Value::Decimal(v))
}
