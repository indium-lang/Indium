# Command-line reference

The executable is `indium` (`indium.exe` on Windows). Source files must have the
lowercase `.in` extension. Quote paths containing spaces. Shell commands belong
in your terminal, not inside a `.in` file.

## Commands

| Command | Purpose |
| --- | --- |
| `indium run FILE.in` | Check the source/import graph, then execute the program |
| `indium check FILE.in` | Check source and local imports without executing the program |
| `indium tokens FILE.in` | Print this file's lexical tokens with line/column locations |
| `indium --version` | Show version and whether the build contains media support |
| `indium --help` | Show a compact usage summary |
| `indium 49` | Small element-name Easter egg |

Running `indium` without arguments, `indium help`, or `indium -h` also shows
help. `indium -V` is a version alias.

`tokens` is useful when inspecting how identifiers, text, punctuation, operators,
and newlines are read. It only lexes the requested file; it does not parse or
execute it, and does not traverse its imports. Its debug token spelling is an
implementation detail rather than a stable machine-readable format.

## Run flags

```text
indium run FILE.in [--max-steps N] [--frames N] [-- ARGS...]
```

| Flag | Meaning |
| --- | --- |
| `--max-steps N` | Stop with a diagnostic once the interpreter exceeds N execution steps |
| `--frames N` | End the scheduler after N graphical frame iterations |
| `--` | Treat every following argument as a text argument for the Indium program |

`N` must be a nonnegative integer. Both limits default to absent. A step is an
internal statement/expression/loop operation, not a CPU instruction or a second
of time. Execution-step limits do not interrupt a blocking input, sleep, network
call, or native drawing call while that call is in progress.

Frame limits apply to the scheduler, after top-level initialization. They are
not a general terminal-program timeout. A terminal program with only timers or
ticks has no frames to count. `--frames 0` performs initialization but dispatches
no scheduler iterations; it does not skip the file's top-level statements.

```powershell
indium run simulation.in --max-steps 100000
indium run drawing.in --frames 120
indium run simulation.in --max-steps 100000 -- first second
```

`check` and `tokens` accept only their source filename, without these run flags.
Unknown commands/options and missing/invalid arguments return an error. There
is no stdin-source mode, inline `-e` evaluator, interactive REPL, project runner,
formatter, or package-manager command implemented in this release.

## Program arguments

```indium run
arguments = args()
if len(arguments) == 0 {
    print("No arguments were supplied.")
} else {
    print("First argument:", arguments[0])
}
```

Run a saved copy with:

```powershell
indium run program.in -- "a name" 49 --custom-option
```

The program receives three text values: `"a name"`, `"49"`, and
`"--custom-option"`. It receives neither the executable name nor source filename.
Convert numeric text explicitly and validate input before relying on it.

## Results and exit status

A successful CLI operation exits with status 0. Argument errors, source-check
errors, and runtime diagnostics exit with status 1 and print the error to
standard error. The `stop()` function ends the program normally; it does not
take an exit-code argument.

A filesystem or HTTP `error(...)` result is ordinary program data. Receiving
one does not by itself make the CLI exit unsuccessfully. Your code decides
whether to print a message, return a result, stop, or raise an assertion.

`--max-steps` is a diagnostic failure when exceeded. Reaching a frame limit is
a normal end of execution. Closing a native window also ends normally.

## Unfinished commands and different kinds of build

`indium build FILE.in` currently reports that native compilation is not
implemented. It does not turn your `.in` source into a standalone executable.

`indium install ...` currently reports that Indium package installation is not
implemented. Built-in modules use `import name`; local modules use a quoted
`.in` path. The Windows setup program installs the interpreter itself and is
separate from this future package manager.

`cargo build --release --locked` builds the **Rust implementation of Indium**.
Its output is the interpreter executable, not a native build of an individual
Indium program. Once the runtime is built, ordinary `indium run` calls do not
need Cargo.
