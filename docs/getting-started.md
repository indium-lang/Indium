# Getting started

Indium programs are plain UTF-8 text files ending in `.in`. The interpreter reads
your file, checks it, and executes its statements.

## 1. Make the command available

On Windows, build Indium once and run `install-windows.cmd`, or use the standalone
`Install-Indium.cmd` with your existing build. The installer copies the runtime
into a permanent per-user folder and adds its `bin` folder to your user PATH.
Close all terminal windows and open a new terminal after installation.

```powershell
indium --version
```

The default build reports `indium 0.1.0-alpha.1 (OpenGL + audio)`. A build made
with `--no-default-features` reports `(terminal-only)` instead. See
[Windows installation](windows-installation.md) for build requirements and the
installer's exact behavior. You do not need Rust every time you run a program.

On a compatible Linux machine, the release ZIP's `indium-linux` launcher can run
the included runtime directly. From the project's main folder:

```sh
./indium-linux run examples/hello.in
```

That launcher is for Linux, not Windows. The README contains the Linux/macOS
source-build instructions and system requirements.

## 2. Create a program

Create a folder for your programs, open a text editor, and save this as
`program.in`:

```indium run
print("Hello, Indium!")
```

Choose UTF-8 text encoding without a byte-order mark (BOM). In Windows Notepad's
Save As dialog, choose **All files** so it does not quietly save
`program.in.txt`. Showing file extensions
in File Explorer makes this easier to spot.

Open a terminal in that folder. One way on Windows is to open the folder in
File Explorer, click the address bar, type `powershell`, and press Enter.

```powershell
indium run program.in
```

Expected output:

```text
Hello, Indium!
```

The command does not create an `.exe` for this program. It executes the `.in`
source through the installed interpreter.

## 3. Understand the two kinds of paths

**The path in the terminal command starts from your terminal's current folder.**
For example, from the main project folder:

```powershell
indium run .\examples\hello.in
```

From inside `examples`, use this instead:

```powershell
indium run .\hello.in
```

PATH lets the terminal find the **Indium executable**. It does not search for
your program file. You can also give an absolute program path:

```powershell
indium run "C:\Users\v\My Programs\program.in"
```

**Paths inside Indium source start from that source file's folder.** If
`games/demo.in` calls `renderer.image("assets/player.png", ...)`, Indium looks
for `games/assets/player.png`, regardless of your terminal's current folder.
Local imports follow the same rule. An imported function uses its own defining
file's folder for its file and asset operations.

Inside an Indium string, use forward slashes in Windows paths or escape each
backslash: `"C:/Users/v/data.txt"` or `"C:\\Users\\v\\data.txt"`.

## 4. Add values and a function

Replace the contents of your file with:

```indium run
name = "flight"
score: integer = 0

fn award(points: integer) -> integer {
    return points * 2
}

for round in 1..4 {
    score += award(round)
}

if score >= 10 {
    print(name, "scored", score)
} else {
    print("Keep going!")
}
```

This prints `flight scored 12`. `1..4` produces 1, 2, and 3; the end is excluded.
`score: integer` declares a type. `+=` updates the existing value. The function
returns twice its argument, and the `if` branch checks a boolean condition.

Newlines separate statements. Braces group blocks. Indentation makes the code
readable, but does not define the blocks. You do not need `let`, `var`, a `main`
function, or a semicolon at the end of each line.

## 5. Read input and arguments

Terminal input reads one line as text:

```indium
name = trim(input("Your name: "))
print("Hello, " + name + "!")
```

For arguments supplied when launching a program:

```indium run
values = args()
print("Number of arguments:", len(values))
for value in values {
    print(value)
}
```

Save that program as `arguments.in`, then:

```powershell
indium run arguments.in -- hello "two words"
```

`args()` returns `["hello", "two words"]` as a list of text values. The `--`
separates interpreter flags from arguments for your program.

## 6. Check a file without running it

```powershell
indium check program.in
```

This parses the program and checks names, known types, control flow, and local
imports. It does not execute `print`, write files, open a window, or make HTTP
requests. Some information is only known at runtime, so a successful check does
not prove every operation will succeed.

If a program fails, read its first error message. It includes the file, line,
column, and usually the source line with a caret. A missing file and a type
mismatch are different problems; [Troubleshooting](troubleshooting.md) explains
the common messages.

## 7. Explore the included demos

From the project's main folder, after installing the command:

```powershell
indium run examples\hello.in
indium run examples\collections.in
indium run examples\results.in
indium run examples\closures.in
indium run examples\2d.in
```

Run each command separately. The 2D demo stays open until you close its window
or press Escape. Its controls are arrow keys for speed, Space for sound, and
F11 for fullscreen. The filesystem, audio, interactive, and HTTP demos have
side effects described in the handbook's example index.

Continue with the [Language chapter](language.md), or jump to
[Graphics, input, and audio](graphics-and-audio.md) to make a window.
