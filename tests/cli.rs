use std::process::Command;
fn cmd(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_indium"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn help_and_version() {
    assert!(cmd(&["--help"]).status.success());
    let r = cmd(&["--version"]);
    assert!(r.status.success());
    assert!(String::from_utf8(r.stdout).unwrap().contains("alpha"));
}
#[test]
fn executes_source_file() {
    let r = cmd(&["run", "examples/hello.in"]);
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    assert_eq!(
        String::from_utf8_lossy(&r.stdout),
        "Hello, Indium!\n1 through 10 adds up to 55\n"
    );
}
#[test]
fn check_does_not_run_and_tokens_work() {
    assert!(cmd(&["check", "examples/hello.in"]).status.success());
    let r = cmd(&["tokens", "examples/hello.in"]);
    assert!(r.status.success());
    assert!(String::from_utf8_lossy(&r.stdout).contains("Ident(\"mode\")"));
}
#[test]
fn failures_are_nonzero_and_explain_the_problem() {
    for args in [
        &["nope"][..],
        &["run"][..],
        &["run", "missing.in"][..],
        &["run", "bad.txt"][..],
        &["run", "examples/hello.in", "--bad"][..],
        &["run", "examples/hello.in", "--frames", "bad"][..],
    ] {
        let r = cmd(args);
        assert!(!r.status.success());
        assert!(!r.stderr.is_empty());
        assert!(!String::from_utf8_lossy(&r.stderr).contains("panicked"));
    }
}
#[test]
fn unsupported_commands_are_explicit() {
    for name in ["build", "install"] {
        let r = cmd(&[name, "example.in"]);
        assert!(!r.status.success());
        assert!(String::from_utf8_lossy(&r.stderr).contains("not implemented"));
    }
}
