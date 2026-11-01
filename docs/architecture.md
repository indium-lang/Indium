# Implementation map

`main.rs` handles CLI arguments and file loading. `loader.rs` canonicalizes paths,
loads/checks the local import graph, detects cycles, and retains source text for
runtime diagnostics. It never executes programs during `check`.

`lexer.rs` produces UTF-8-aware tokens with byte offsets and line/column spans.
`parser.rs` uses a Pratt expression parser and structured statement parser.
`ast.rs` defines the syntax tree, basic type vocabulary, and recurring events.

`checker.rs` checks lexical names, known scalar types, annotations, function
arity, scope rules, and control-flow placement. It hoists function declarations
and checks their bodies after the enclosing declarations. Dynamic values remain
runtime-checked; the pass is intentionally not described as a complete static
type inference system.

`interpreter.rs` executes the AST. It implements lexical closures, functions,
control flow, checked arithmetic, indexed assignment, module caching, and source
error stacks. Assignment targets are evaluated once before the right-hand side.
The runtime uses reference-counted lexical environments and collection values.
It has instruction, nesting, evaluation, and call guards rather than relying on
host stack overflows or raw Rust panics for normal program errors.

`runtime/scheduler.rs` drives independent tick and render deadlines, with bounded
catch-up and timer cadence. Handlers share lexical state and run on one thread.
The fixed-step accumulator is also tested independently from wall-clock time.

`stdlib` exposes built-ins and modules through a consistent value/call interface.
File/HTTP operational failures use explicit success/error results. Networking
currently launches curl with individual arguments, not shell commands.

`media/backend.rs` uses rust-sdl2 to create an actual OpenGL renderer. CPU-decoded
images and font rasters are cached; GPU resources created for a draw are scoped
and freed. The SDL offscreen resize path rebuilds its pbuffer-backed window;
native desktop drivers preserve their context on resize.

`media/audio.rs` uses Symphonia to decode OGG/Vorbis and PCM WAV into cached audio
clips. An SDL audio callback mixes voices with per-voice gain, pause/loop state,
and linear sample-rate conversion. Audio uses an internal callback thread while
language code runs on the runtime thread; shared mixer state is protected by a
mutex. The device is opened only on demand and paused before destruction.





## Rust library entry points

The project is both a CLI binary and a Rust library crate. `src/lib.rs` exports
the lexer, parser, checker, loader, interpreter, runtime, media, and standard
library modules. These are pre-alpha implementation APIs, not a stable embedding
contract.

The convenience helper `indium::run_source(source)` checks a source string,
executes it, captures normal terminal output, and returns `Result<String>`.
It sets a one-million-step execution budget. It is used throughout the language
tests:

```rust
let output = indium::run_source("print(49)\n").unwrap();
assert_eq!(output, "49\n");
```

Local file imports need a real defining file path, so `run_source` rejects
programs containing them. For file-backed execution, use
`loader::compile_file(Path::new("program.in"))`, create an
`interpreter::Interpreter` with that program and `interpreter::Options`, and call
its `run()` method. The options control output capture and step/frame limits.
The loader's `compile_file` name means parsing/checking into this interpreter's
program representation; it does not generate machine code.

Diagnostics retain source spans, filenames, messages, and call notes. `render`
can combine one with source text to produce the same kind of caret display the
CLI uses. Error results produced by Indium programs are separate language values,
not Rust loader/interpreter diagnostics.

## Maintaining the handbook

Edit the Markdown files in `docs/` and keep examples aligned with the current
implementation. The documentation checker compares exported callable/property
names with the guide, verifies local links, checks complete code blocks, runs
the explicitly marked safe blocks, and checks all shipped example files.
Interactive, filesystem-writing, and public-network snippets are not run
automatically; the new interactive game gets controlled test input separately.

```sh
python3 scripts/check-docs.py --graphics
```

`--graphics` runs marked drawing examples with offscreen Mesa and a dummy audio
device. Omit it on a system without that headless driver; the graphics source
is still checked. Pass `--interpreter PATH` to choose an executable,
`--verbose` to inspect outputs, or `--report PATH.json` to retain a report.

The HTML builder needs Node.js and the `marked` Markdown renderer. Runtime use
of Indium and reading the already-built handbook need neither:

```sh
npm install --prefix .doc-tools --no-save --no-package-lock marked
node scripts/build-guide.mjs
python3 scripts/package.py --output-dir /path/to/output-folder
```

The builder combines the chapters into `docs/guide.html`, embeds its styles,
rewrites chapter links to internal anchors, and validates those anchors. The
packager includes that generated file with the Markdown source, examples,
existing Linux binary, Windows setup helpers, local Git bundle, and checksums.
