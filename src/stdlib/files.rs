use super::Args;
use crate::{diagnostic::Result, interpreter::Interpreter, runtime::values::Value};
use std::io::Write;
pub fn call(i: &mut Interpreter, n: &str, a: Args<'_>) -> Result<Value> {
    a.exact(i, if n == "write" || n == "append" { 2 } else { 1 })?;
    let path = i.asset_path(a.text(i, 0)?);
    let content = if n == "write" || n == "append" {
        Some(a.text(i, 1)?.to_owned())
    } else {
        None
    };
    if n == "exists" {
        return Ok(Value::Boolean(path.exists()));
    }
    let result: std::result::Result<Value, std::io::Error> = match n {
        "read" => std::fs::read_to_string(path).map(Value::Text),
        "write" => std::fs::write(path, content.as_ref().unwrap()).map(|_| Value::None),
        "append" => std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .and_then(|mut f| f.write_all(content.as_ref().unwrap().as_bytes()))
            .map(|_| Value::None),
        "mkdir" => std::fs::create_dir_all(path).map(|_| Value::None),
        "remove" => std::fs::remove_file(path).map(|_| Value::None),
        "list" => std::fs::read_dir(path).and_then(|entries| {
            let mut names = Vec::new();
            for entry in entries {
                names.push(entry?.file_name().to_string_lossy().into_owned());
            }
            names.sort();
            Ok(Value::list(names.into_iter().map(Value::Text).collect()))
        }),
        _ => return Err(i.error(format!("Unknown files function '{n}'"), a.span)),
    };
    Ok(match result {
        Ok(v) => Value::outcome(true, v),
        Err(e) => Value::outcome(false, Value::Text(e.to_string())),
    })
}
