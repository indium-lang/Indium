use super::Args;
use crate::{diagnostic::Result, interpreter::Interpreter, runtime::values::Value};
use std::{
    collections::BTreeMap,
    io::Write,
    process::{Command, Stdio},
};
pub fn call(i: &mut Interpreter, n: &str, a: Args<'_>) -> Result<Value> {
    a.exact(i, if n == "post" { 2 } else { 1 })?;
    let url = a.text(i, 0)?;
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(i.error("HTTP URLs must start with https:// or http://", a.span));
    }
    let mut command = Command::new(if cfg!(windows) { "curl.exe" } else { "curl" });
    command.args([
        "--disable",
        "--silent",
        "--show-error",
        "--max-time",
        "15",
        "--max-filesize",
        "16777216",
        "--proto",
        "=http,https",
        "--write-out",
        "\n%{http_code}",
    ]);
    let mut body = String::new();
    if n == "post" {
        body = a.text(i, 1)?.into();
        command.args(["--request", "POST", "--data-binary", "@-"]);
    }
    command
        .arg("--url")
        .arg(url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let result = (|| -> std::result::Result<Value, String> {
        let mut child = command
            .spawn()
            .map_err(|e| format!("HTTP needs curl on PATH: {e}"))?;
        let write_result = if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(body.as_bytes())
        } else {
            Ok(())
        };
        let output = child.wait_with_output().map_err(|e| e.to_string())?;
        write_result.map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().into());
        }
        let raw = String::from_utf8(output.stdout).map_err(|_| "HTTP body is not UTF-8 text")?;
        let (body, status) = raw.rsplit_once('\n').ok_or("Invalid HTTP response")?;
        let status = status.parse::<i64>().map_err(|_| "Invalid HTTP status")?;
        Ok(Value::map(BTreeMap::from([
            ("status".into(), Value::Integer(status)),
            ("body".into(), Value::Text(body.into())),
        ])))
    })();
    Ok(match result {
        Ok(v) => Value::outcome(true, v),
        Err(e) => Value::outcome(false, Value::Text(e)),
    })
}
