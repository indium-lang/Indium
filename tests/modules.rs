use indium::{
    interpreter::{Interpreter, Options},
    loader,
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Dir(PathBuf);
impl Dir {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "indium-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
    fn write(&self, name: &str, source: &str) {
        fs::write(self.0.join(name), source).unwrap();
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn run(d: &Dir) -> String {
    let p = loader::compile_file(&d.0.join("main.in")).unwrap();
    let mut i = Interpreter::new(
        p,
        Options {
            capture_output: true,
            max_steps: Some(10000),
            max_frames: None,
        },
    );
    i.run().unwrap();
    i.output
}
#[test]
fn local_imports_are_cached_and_functions_keep_lexical_state() {
    let d = Dir::new();
    d.write(
        "counter.in",
        "count=0\nprint(\"loaded\")\nfn next(){count+=1\nreturn count}\n_private=49",
    );
    d.write("main.in","import \"counter.in\" as a\nimport \"counter.in\" as b\nprint(a.next(), b.next(), a.count)");
    assert_eq!(run(&d), "loaded\n1 2 2\n");
}
#[test]
fn circular_imports_are_reported() {
    let d = Dir::new();
    d.write("a.in", "import \"b.in\"");
    d.write("b.in", "import \"a.in\"");
    let e = loader::compile_file(&d.0.join("a.in")).err().unwrap();
    assert!(e.message.contains("Circular import"));
}
#[test]
fn missing_import_has_the_import_location() {
    let d = Dir::new();
    d.write("main.in", "print(1)\nimport \"missing.in\"");
    let e = loader::compile_file(&d.0.join("main.in")).err().unwrap();
    assert_eq!(e.span.line, 2);
    assert!(e.file.ends_with("main.in"));
}
#[test]
fn imported_diagnostic_points_to_module() {
    let d = Dir::new();
    d.write("bad.in", "x=1\nprint(unknown)");
    d.write("main.in", "import \"bad.in\"");
    let e = loader::compile_file(&d.0.join("main.in")).err().unwrap();
    assert!(e.file.ends_with("bad.in"));
    assert_eq!(e.span.line, 2);
}
#[test]
fn files_use_paths_relative_to_the_source() {
    let d = Dir::new();
    d.write("main.in","import files\nx=files.write(\"note.txt\",\"hello\")\nassert(x is success)\nx=files.append(\"note.txt\",\" world\")\nassert(x is success)\nread=files.read(\"note.txt\")\nprint(read.value, files.exists(\"note.txt\"))\nmissing=files.read(\"not-found\")\nprint(missing is error)");
    assert_eq!(run(&d), "hello world true\ntrue\n");
    assert_eq!(
        fs::read_to_string(d.0.join("note.txt")).unwrap(),
        "hello world"
    );
}
#[test]
fn file_argument_type_errors_do_not_panic() {
    let d = Dir::new();
    d.write("main.in", "import files\nfiles.append(\"data\",123)");
    let p = loader::compile_file(&d.0.join("main.in")).unwrap();
    let mut i = Interpreter::new(p, Options::default());
    assert!(i.run().unwrap_err().message.contains("must be text"));
}
