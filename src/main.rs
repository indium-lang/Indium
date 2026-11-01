use indium::{
    interpreter::{Interpreter, Options},
    loader,
};
use std::{path::Path, process::ExitCode};
const HELP:&str="Indium 0.1.0-alpha.1 — pre-alpha interpreter\n\nUsage:\n  indium run FILE.in [--max-steps N] [--frames N] [-- ARGS...]\n  indium check FILE.in\n  indium tokens FILE.in\n  indium --version\n  indium --help\n\nrun      Execute a real Indium program\ncheck    Parse and check the complete local module graph without executing it\ntokens   Print lexer tokens with source locations\n\nGraphics and audio are included in default builds.\nNative build and package installation are not implemented yet.\n";
fn main() -> ExitCode {
    match cli() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}
fn cli() -> std::result::Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.is_empty() || matches!(args[0].as_str(), "--help" | "-h" | "help") {
        print!("{HELP}");
        return Ok(());
    }
    if matches!(args[0].as_str(), "--version" | "-V") {
        println!(
            "indium {}{}",
            env!("CARGO_PKG_VERSION"),
            if cfg!(feature = "media") {
                " (OpenGL + audio)"
            } else {
                " (terminal-only)"
            }
        );
        return Ok(());
    }
    if args[0] == "49" {
        println!("Indium · In · atomic number 49");
        return Ok(());
    }
    if args[0] == "build" {
        return Err("Native compilation is not implemented yet. Use 'indium run FILE.in'.".into());
    }
    if args[0] == "install" {
        return Err("Package installation is not implemented yet. Built-in modules use 'import name'; local modules use 'import \"path.in\"'.".into());
    }
    if !matches!(args[0].as_str(), "run" | "check" | "tokens") {
        return Err(format!(
            "Unknown command '{}'. Use 'indium --help'.",
            args[0]
        ));
    }
    let file = args
        .get(1)
        .ok_or_else(|| format!("Expected a .in source file after '{}'.", args[0]))?;
    if Path::new(file).extension().and_then(|s| s.to_str()) != Some("in") {
        return Err("Indium source files must use the .in extension.".into());
    }
    let mut options = Options::default();
    let mut script_args = Vec::new();
    let mut index = 2;
    while index < args.len() {
        let arg = &args[index];
        if args[0] != "run" {
            return Err(format!("Unexpected argument '{arg}'."));
        }
        if arg == "--" {
            script_args.extend_from_slice(&args[index + 1..]);
            break;
        }
        if arg == "--frames" || arg == "--max-steps" {
            let number = args
                .get(index + 1)
                .ok_or_else(|| format!("Expected a nonnegative integer after '{arg}'."))?
                .parse::<u64>()
                .map_err(|_| format!("Expected a nonnegative integer after '{arg}'."))?;
            if arg == "--frames" {
                options.max_frames = Some(number)
            } else {
                options.max_steps = Some(number)
            }
            index += 2;
        } else {
            return Err(format!(
                "Unknown option '{arg}'. Script arguments go after '--'."
            ));
        }
    }
    if args[0] == "tokens" {
        let source =
            std::fs::read_to_string(file).map_err(|e| format!("Cannot read '{file}': {e}"))?;
        for t in indium::lexer::lex(&source).map_err(|e| e.in_file(file).render(Some(&source)))? {
            println!("{}:{} {:?}", t.span.line, t.span.column, t.kind);
        }
        return Ok(());
    }
    let p = loader::compile_file(Path::new(file)).map_err(|e| {
        let source = std::fs::read_to_string(&e.file).ok();
        e.render(source.as_deref())
    })?;
    if args[0] == "check" {
        println!("Checked {} module(s): {file}", p.modules.len());
        return Ok(());
    }
    let mut interpreter = Interpreter::new(p, options);
    interpreter.args = script_args;
    interpreter
        .run()
        .map_err(|e| e.render(interpreter.program.sources.get(&e.file).map(String::as_str)))
}
