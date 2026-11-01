pub mod ast;
pub mod checker;
pub mod diagnostic;
pub mod interpreter;
pub mod lexer;
pub mod loader;
pub mod media;
pub mod parser;
pub mod runtime;
pub mod stdlib;

pub fn run_source(source: &str) -> diagnostic::Result<String> {
    let p = loader::compile_source(source)?;
    let options = interpreter::Options {
        capture_output: true,
        max_steps: Some(1_000_000),
        max_frames: None,
    };
    let mut i = interpreter::Interpreter::new(p, options);
    i.run()?;
    Ok(i.output)
}
