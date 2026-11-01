use crate::{
    ast::Type,
    diagnostic::{Result, Span},
    interpreter::Interpreter,
    runtime::values::Value,
};
use std::{
    collections::BTreeMap,
    io::{self, Write},
    time::Duration,
};
mod files;
mod math;
mod net;

pub fn initial_values() -> BTreeMap<String, Value> {
    let mut m = BTreeMap::new();
    for n in [
        "print", "write", "len", "assert", "type_of", "push", "pop", "keys", "contains", "split",
        "join", "replace", "trim", "upper", "lower", "stop", "sleep", "args",
    ] {
        m.insert(n.into(), Value::Builtin(n.into()));
    }
    for n in ["window", "renderer", "input", "audio"] {
        m.insert(n.into(), Value::Namespace(n.into()));
    }
    for n in [
        "integer", "decimal", "text", "boolean", "list", "map", "function", "any",
    ] {
        m.insert(n.into(), Value::Type(Type::parse(n).unwrap()));
    }
    for n in ["terminal", "background", "2d", "3d"] {
        m.insert(n.into(), Value::Mode(n.into()));
    }
    m.insert("success".into(), Value::State(true));
    m.insert("error".into(), Value::State(false));
    m
}
pub fn has_module(n: &str) -> bool {
    matches!(n, "math" | "files" | "net" | "time" | "game" | "text")
}
pub fn call(i: &mut Interpreter, n: &str, args: Vec<Value>, s: Span) -> Result<Value> {
    let a = Args {
        values: &args,
        span: s,
    };
    if let Some(name) = n.strip_prefix("math.") {
        return math::call(i, name, a);
    }
    if let Some(name) = n.strip_prefix("files.") {
        return files::call(i, name, a);
    }
    if let Some(name) = n.strip_prefix("net.") {
        return net::call(i, name, a);
    }
    match n {
        "print" | "write" => {
            let mut out = args
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            if n == "print" {
                out.push('\n');
            }
            if i.options.capture_output {
                i.output.push_str(&out);
            } else {
                let mut stdout = io::stdout().lock();
                stdout
                    .write_all(out.as_bytes())
                    .and_then(|_| stdout.flush())
                    .map_err(|e| i.error(format!("Cannot write terminal output: {e}"), s))?;
            }
            Ok(Value::None)
        }
        "input" | "input.read" => {
            a.between(i, 0, 1)?;
            if let Some(prompt) = args.first() {
                let out = prompt.to_string();
                if i.options.capture_output {
                    i.output.push_str(&out);
                } else {
                    print!("{out}");
                    io::stdout()
                        .flush()
                        .map_err(|e| i.error(format!("Cannot write prompt: {e}"), s))?;
                }
            }
            let mut line = String::new();
            io::stdin()
                .read_line(&mut line)
                .map_err(|e| i.error(format!("Cannot read input: {e}"), s))?;
            Ok(Value::Text(line.trim_end_matches(['\r', '\n']).into()))
        }
        "len" | "text.len" => {
            a.exact(i, 1)?;
            let len = match &args[0] {
                Value::Text(s) => s.chars().count(),
                Value::List(x) => x.borrow().len(),
                Value::Map(m) => m.borrow().len(),
                Value::Range(a, b) => usize::try_from((*b as i128 - *a as i128).max(0))
                    .map_err(|_| i.error("Range length is too large", s))?,
                _ => return Err(i.error("len needs text, a list, map, or range", s)),
            };
            let n =
                i64::try_from(len).map_err(|_| i.error("Length exceeds the integer range", s))?;
            Ok(Value::Integer(n))
        }
        "type_of" => {
            a.exact(i, 1)?;
            Ok(Value::Text(args[0].type_name().into()))
        }
        "assert" => {
            a.between(i, 1, 2)?;
            if !i.boolean(args[0].clone(), s)? {
                return Err(i.error(
                    args.get(1)
                        .map(ToString::to_string)
                        .unwrap_or("Assertion failed".into()),
                    s,
                ));
            }
            Ok(Value::None)
        }
        "push" => {
            a.exact(i, 2)?;
            if let Value::List(xs) = &args[0] {
                xs.borrow_mut().push(args[1].clone());
                Ok(Value::None)
            } else {
                Err(i.error("push expects a list and a value", s))
            }
        }
        "pop" => {
            a.exact(i, 1)?;
            if let Value::List(xs) = &args[0] {
                Ok(xs.borrow_mut().pop().unwrap_or(Value::None))
            } else {
                Err(i.error("pop expects a list", s))
            }
        }
        "keys" => {
            a.exact(i, 1)?;
            if let Value::Map(m) = &args[0] {
                Ok(Value::list(
                    m.borrow().keys().cloned().map(Value::Text).collect(),
                ))
            } else {
                Err(i.error("keys expects a map", s))
            }
        }
        "contains" | "text.contains" => {
            a.exact(i, 2)?;
            let yes = match &args[0] {
                Value::Text(text) => text.contains(a.text(i, 1)?),
                Value::List(xs) => xs.borrow().iter().any(|x| x.equals(&args[1])),
                Value::Map(m) => m.borrow().contains_key(a.text(i, 1)?),
                _ => {
                    return Err(i.error(
                        "contains expects text, list, or map as its first argument",
                        s,
                    ))
                }
            };
            Ok(Value::Boolean(yes))
        }
        "split" | "text.split" => {
            a.exact(i, 2)?;
            Ok(Value::list(
                a.text(i, 0)?
                    .split(a.text(i, 1)?)
                    .map(|s| Value::Text(s.into()))
                    .collect(),
            ))
        }
        "join" | "text.join" => {
            a.exact(i, 2)?;
            if let Value::List(xs) = &args[0] {
                Ok(Value::Text(
                    xs.borrow()
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(a.text(i, 1)?),
                ))
            } else {
                Err(i.error("join expects a list and a text separator", s))
            }
        }
        "replace" | "text.replace" => {
            a.exact(i, 3)?;
            Ok(Value::Text(
                a.text(i, 0)?.replace(a.text(i, 1)?, a.text(i, 2)?),
            ))
        }
        "trim" | "text.trim" => {
            a.exact(i, 1)?;
            Ok(Value::Text(a.text(i, 0)?.trim().into()))
        }
        "upper" | "text.upper" => {
            a.exact(i, 1)?;
            Ok(Value::Text(a.text(i, 0)?.to_uppercase()))
        }
        "lower" | "text.lower" => {
            a.exact(i, 1)?;
            Ok(Value::Text(a.text(i, 0)?.to_lowercase()))
        }
        "stop" | "window.close" | "game.stop" => {
            a.exact(i, 0)?;
            i.stopped = true;
            Ok(Value::None)
        }
        "sleep" | "time.sleep" => {
            a.exact(i, 1)?;
            let n = a.number(i, 0)?;
            if !(0.0..=86400.0).contains(&n) {
                return Err(i.error("sleep duration must be between 0 and 86400 seconds", s));
            }
            std::thread::sleep(Duration::from_secs_f64(n));
            Ok(Value::None)
        }
        "time.now" => {
            a.exact(i, 0)?;
            Ok(Value::Decimal(i.started.elapsed().as_secs_f64()))
        }
        "args" => {
            a.exact(i, 0)?;
            Ok(Value::list(
                i.args.iter().cloned().map(Value::Text).collect(),
            ))
        }
        "window.toggle_fullscreen" => {
            a.exact(i, 0)?;
            let v = Value::Boolean(!i.media.config.fullscreen);
            set_member(i, "window", "fullscreen", v, s)?;
            Ok(Value::None)
        }
        n if n.starts_with("renderer.")
            || n.starts_with("input.")
            || n.starts_with("audio.")
            || n == "audio" =>
        {
            media_call(i, n, a)
        }
        _ => Err(i.error(format!("Unknown function '{n}'"), s)),
    }
}
pub fn member(i: &mut Interpreter, n: &str, key: &str, s: Span) -> Result<Value> {
    let full = format!("{n}.{key}");
    let value = match (n, key) {
        ("math", "pi") => Value::Decimal(std::f64::consts::PI),
        ("math", "e") => Value::Decimal(std::f64::consts::E),
        ("math", "tau") => Value::Decimal(std::f64::consts::TAU),
        ("window", "title") => Value::Text(i.media.config.title.clone()),
        ("window", "width") => Value::Integer(i.media.config.width as i64),
        ("window", "height") => Value::Integer(i.media.config.height as i64),
        ("window", "fps") => Value::Integer(i.media.config.fps as i64),
        ("window", "vsync") => Value::Boolean(i.media.config.vsync),
        ("window", "fullscreen") => Value::Boolean(i.media.config.fullscreen),
        ("window", "exclusive") => Value::Boolean(i.media.config.exclusive),
        ("window", "backend") => Value::Text(i.media.backend()),
        ("game", "tps") => Value::Decimal(i.tps),
        ("game", "dt") => Value::Decimal(i.tick_dt),
        ("game", "ticks") => Value::Integer(i.tick_count),
        ("game", "frames") => Value::Integer(i.frame_count),
        ("game", "max_catchup") => Value::Integer(i.max_catchup as i64),
        ("game", "dropped_ticks") => Value::Integer(i.dropped_ticks.min(i64::MAX as u64) as i64),
        ("time", "dt") => Value::Decimal(i.frame_dt),
        ("time", "elapsed") => Value::Decimal(i.started.elapsed().as_secs_f64()),
        ("input", "mouse_x") => Value::Integer(i.media.mouse().0 as i64),
        ("input", "mouse_y") => Value::Integer(i.media.mouse().1 as i64),
        _ if known_function(&full) => Value::Builtin(full),
        _ => return Err(i.error(format!("Module '{n}' has no member '{key}'"), s)),
    };
    Ok(value)
}
fn known_function(n: &str) -> bool {
    matches!(
        n,
        "math.sqrt"
            | "math.abs"
            | "math.floor"
            | "math.ceil"
            | "math.round"
            | "math.sin"
            | "math.cos"
            | "math.tan"
            | "math.log"
            | "math.exp"
            | "math.pow"
            | "math.min"
            | "math.max"
            | "math.clamp"
            | "math.random"
            | "math.random_int"
            | "math.seed"
            | "files.read"
            | "files.write"
            | "files.append"
            | "files.exists"
            | "files.list"
            | "files.mkdir"
            | "files.remove"
            | "net.get"
            | "net.post"
            | "time.now"
            | "time.sleep"
            | "game.stop"
            | "window.close"
            | "window.toggle_fullscreen"
            | "renderer.clear"
            | "renderer.rect"
            | "renderer.outline"
            | "renderer.line"
            | "renderer.circle"
            | "renderer.text"
            | "renderer.image"
            | "renderer.save"
            | "input.read"
            | "input.key_down"
            | "input.key_pressed"
            | "input.mouse_down"
            | "input.mouse_pressed"
            | "audio.play"
            | "audio.pause"
            | "audio.resume"
            | "audio.stop"
            | "audio.volume"
            | "audio.playing"
            | "text.len"
            | "text.contains"
            | "text.split"
            | "text.join"
            | "text.replace"
            | "text.trim"
            | "text.upper"
            | "text.lower"
    )
}
pub fn set_member(i: &mut Interpreter, n: &str, key: &str, v: Value, s: Span) -> Result<()> {
    if n == "window" {
        let mut c = i.media.config.clone();
        match key {
            "title" => {
                if let Value::Text(t) = v {
                    c.title = t
                } else {
                    return Err(i.error("window.title must be text", s));
                }
            }
            "width" | "height" => {
                let x = integer(&v)
                    .filter(|x| (1..=16384).contains(x))
                    .ok_or_else(|| {
                        i.error("Window dimensions must be integers between 1 and 16384", s)
                    })?;
                if key == "width" {
                    c.width = x as u32
                } else {
                    c.height = x as u32
                }
            }
            "fps" => {
                c.fps = integer(&v)
                    .filter(|x| (0..=10000).contains(x))
                    .ok_or_else(|| {
                        i.error(
                            "window.fps must be an integer between 0 (uncapped) and 10000",
                            s,
                        )
                    })? as u32;
            }
            "fullscreen" | "exclusive" | "vsync" => {
                let x = i.boolean(v, s)?;
                match key {
                    "fullscreen" => c.fullscreen = x,
                    "exclusive" => c.exclusive = x,
                    _ => c.vsync = x,
                }
            }
            _ => return Err(i.error(format!("Unknown or read-only window property '{key}'"), s)),
        }
        return i.media.configure(c).map_err(|e| i.error(e, s));
    }
    if n == "game" {
        match key {
            "tps" => {
                let x = i.number(&v, s)?;
                if !(1.0..=10000.0).contains(&x) {
                    return Err(i.error("game.tps must be between 1 and 10000", s));
                }
                i.tps = x;
                i.tick_dt = 1.0 / x;
            }
            "max_catchup" => {
                i.max_catchup = integer(&v)
                    .filter(|x| (1..=1000).contains(x))
                    .ok_or_else(|| {
                        i.error("game.max_catchup must be an integer between 1 and 1000", s)
                    })? as usize;
            }
            _ => return Err(i.error(format!("Unknown or read-only game property '{key}'"), s)),
        }
        return Ok(());
    }
    Err(i.error(format!("Cannot assign '{n}.{key}'"), s))
}
pub fn convert(i: &Interpreter, t: &Type, args: Vec<Value>, s: Span) -> Result<Value> {
    if args.len() != 1 {
        return Err(i.error(format!("{}() expects one argument", t.name()), s));
    }
    let v = args.into_iter().next().unwrap();
    let bad = || i.error(format!("Cannot convert value to {}", t.name()), s);
    match t {
        Type::Text => Ok(Value::Text(v.to_string())),
        Type::Integer => match v {
            Value::Integer(n) => Ok(Value::Integer(n)),
            Value::Decimal(n) if n.is_finite() && n >= i64::MIN as f64 && n < i64::MAX as f64 => {
                Ok(Value::Integer(n.trunc() as i64))
            }
            Value::Text(s) => s.trim().parse().map(Value::Integer).map_err(|_| bad()),
            Value::Boolean(b) => Ok(Value::Integer(i64::from(b))),
            _ => Err(bad()),
        },
        Type::Decimal => {
            let n = match v {
                Value::Integer(n) => n as f64,
                Value::Decimal(n) => n,
                Value::Text(s) => s.trim().parse().map_err(|_| bad())?,
                _ => return Err(bad()),
            };
            if n.is_finite() {
                Ok(Value::Decimal(n))
            } else {
                Err(bad())
            }
        }
        Type::Boolean => match v {
            Value::Boolean(b) => Ok(Value::Boolean(b)),
            Value::Text(s) if s == "true" => Ok(Value::Boolean(true)),
            Value::Text(s) if s == "false" => Ok(Value::Boolean(false)),
            _ => Err(bad()),
        },
        Type::Any => Ok(v),
        t if v.ty() == *t => Ok(v),
        _ => Err(bad()),
    }
}
fn integer(v: &Value) -> Option<i64> {
    if let Value::Integer(n) = v {
        Some(*n)
    } else {
        None
    }
}
pub struct Args<'a> {
    pub values: &'a [Value],
    pub span: Span,
}
impl<'a> Args<'a> {
    pub fn exact(&self, i: &Interpreter, n: usize) -> Result<()> {
        self.between(i, n, n)
    }
    pub fn between(&self, i: &Interpreter, min: usize, max: usize) -> Result<()> {
        if self.values.len() < min || self.values.len() > max {
            Err(i.error(
                format!(
                    "Expected {} arguments, got {}",
                    if min == max {
                        min.to_string()
                    } else {
                        format!("{min}..{max}")
                    },
                    self.values.len()
                ),
                self.span,
            ))
        } else {
            Ok(())
        }
    }
    pub fn text(&self, i: &Interpreter, n: usize) -> Result<&'a str> {
        match self.values.get(n) {
            Some(Value::Text(s)) => Ok(s),
            _ => Err(i.error(format!("Argument {} must be text", n + 1), self.span)),
        }
    }
    pub fn number(&self, i: &Interpreter, n: usize) -> Result<f64> {
        self.values
            .get(n)
            .and_then(Value::number)
            .ok_or_else(|| i.error(format!("Argument {} must be a number", n + 1), self.span))
    }
    pub fn int(&self, i: &Interpreter, n: usize) -> Result<i64> {
        self.values
            .get(n)
            .and_then(integer)
            .ok_or_else(|| i.error(format!("Argument {} must be an integer", n + 1), self.span))
    }
    pub fn boolean(&self, i: &Interpreter, n: usize) -> Result<bool> {
        if let Some(Value::Boolean(b)) = self.values.get(n) {
            Ok(*b)
        } else {
            Err(i.error(format!("Argument {} must be boolean", n + 1), self.span))
        }
    }
}
fn media_call(i: &mut Interpreter, n: &str, a: Args<'_>) -> Result<Value> {
    if n == "audio" || n == "audio.play" {
        a.between(i, 1, 3)?;
        let path = i.asset_path(a.text(i, 0)?);
        let volume = if a.values.len() > 1 {
            a.number(i, 1)?
        } else {
            1.0
        };
        let looping = if a.values.len() > 2 {
            a.boolean(i, 2)?
        } else {
            false
        };
        if !(0.0..=1.0).contains(&volume) {
            return Err(i.error("Volume must be between 0 and 1", a.span));
        }
        return i
            .media
            .play_audio(&path, volume as f32, looping)
            .map(|n| Value::Integer(n as i64))
            .map_err(|e| i.error(e, a.span));
    }
    if n.starts_with("audio.") {
        a.between(i, 1, if n == "audio.volume" { 2 } else { 1 })?;
        let id = a.int(i, 0)?;
        let volume = if n == "audio.volume" {
            a.exact(i, 2)?;
            let x = a.number(i, 1)?;
            if !(0.0..=1.0).contains(&x) {
                return Err(i.error("Volume must be between 0 and 1", a.span));
            }
            Some(x as f32)
        } else {
            None
        };
        return i
            .media
            .audio_control(n.trim_start_matches("audio."), id, volume)
            .map(Value::Boolean)
            .map_err(|e| i.error(e, a.span));
    }
    if n.starts_with("input.") {
        a.exact(i, 1)?;
        let key = a.text(i, 0)?;
        return i
            .media
            .input(n.trim_start_matches("input."), key)
            .map(Value::Boolean)
            .map_err(|e| i.error(e, a.span));
    }
    if !i.mode_2d() {
        return Err(i.error("Drawing requires mode = 2d", a.span));
    }
    // Drawing occurs inside frame handlers, after the window and OpenGL context exist.
    let result = i
        .media
        .draw(n.trim_start_matches("renderer."), a.values, &i.file);
    result.map(|_| Value::None).map_err(|e| i.error(e, a.span))
}
