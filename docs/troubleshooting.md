# Troubleshooting

Start with the **first complete error**, including its filename and line number.
An installation error happens before a program runs; a language diagnostic
refers to your `.in` source; an error result is data returned to your program.

## The terminal cannot find indium

If the message says `indium` is not recognized, the terminal cannot find the
executable through its PATH. This does not indicate a syntax problem in your
program. Run the Windows installer, close all terminal windows, and open a new
terminal. Existing terminal host processes may retain their old environment.
If necessary, sign out of Windows and sign back in.

Verify with `indium --version` before trying a source file. For a direct check
of a source build, run this from the **main project folder** containing Cargo.toml:

```powershell
.\target\release\indium.exe --version
```

That relative path is wrong from `C:\Users\v` or from inside `examples`. If there
is no `target\release\indium.exe` in the project, the Windows build has not
completed. Installing Rust alone does not build or install Indium.

## The source file cannot be found

Your terminal's current folder determines the path in `indium run ...`. From
the project root, use `examples\hello.in`; from inside `examples`, use `hello.in`.
Quote paths containing spaces. Check that an editor did not add `.txt` after
the `.in` extension. The CLI expects lowercase `.in`.

If the first character produces an unexpected-character error, check the text
encoding. Save UTF-8 **without BOM**; the lexer does not strip a byte-order mark,
and UTF-16 files are not supported.

Once the source starts, file/import/asset paths inside the program are relative
to its defining `.in` file, not to the terminal's folder. Keep the demo assets
and local modules in the supplied folder structure.

## Windows build and DLL problems

| Message | Meaning and next step |
| --- | --- |
| `cargo` is not recognized | Rust/Cargo is missing from this terminal; install Rust or reopen the terminal after installing it |
| Microsoft linker/build tools are missing | Install the MSVC C++ toolchain, including Desktop development with C++ |
| `SDL2.lib` was not found in an x64 folder | Use the SDL2 Visual C++ **development** package and point the helper at its extracted folder |
| `LNK1181: cannot open input file 'SDL2.lib'` | The linker has no usable SDL2 search path; use the updated helper with encoded flags |
| Windows reports missing `SDL2.dll` | Keep the x64 SDL2 DLL beside `indium.exe`; the build helper and installer copy it |
| Access denied while replacing `indium.exe` | Close Indium processes before rebuilding, updating, or uninstalling |

The original build helper incorrectly put literal quotes in `RUSTFLAGS`. The
updated script uses `CARGO_ENCODED_RUSTFLAGS` to preserve paths with spaces.
Download the updated project if you still have the original helper. Do not
delete your working Windows build just to replace documentation files.

The release ZIP's prebuilt Linux runtime is not a Windows executable. The
standalone Windows setup installs an already-built Windows runtime; it does
not compile one or download a replacement in the background.

## Unknown variable or read-only name

Define a variable before using it, check spelling/capitalization, and consider
scope. A variable created inside a block is not visible outside that block.
Function declarations are hoisted, but captured variables still initialize in
source order. Handler state should be declared before the handler.

Built-ins and imports have read-only bindings. Use a different variable name
instead of assigning to `print`, `input`, `text`, `math`, or another imported
alias. A declared function name cannot be reassigned either.

## Expected integer, got decimal

The first assignment normally fixes a variable's runtime type. Division always
produces a decimal, and functions such as `math.floor` also return decimals.

Deliberately invalid example:

```indium error
count = 10
count = count / 2
```

Use a decimal binding if fractions are part of the program:

```indium run
count = 10.0
count = count / 2
print(count)
```

Or explicitly convert a result with `integer(...)` when truncation is intended.
Use `name: any = ...` only when changing types is intentional. Applying
`any(value)` alone does not change a variable's declared type.

## Conditions require booleans

This fails because a number is not a boolean:

```indium error
if 1 { print("not reached") }
```

Write `if count > 0`, `if len(items) > 0`, or an explicit boolean condition.
`boolean(1)` is not a truthiness conversion. Also use `==` for equality;
`is` is for a type, `none`, or a result state.

## Result has the other state

You accessed `.value` on an error, or `.error` on a success:

```indium error
result = error("not found")
print(result.value)
```

Check `result is success`, `result is error`, or `result.ok` first. Do not apply
`is error` to an arbitrary integer/text value: state checks require a result.
An invalid conversion or media operation is usually a diagnostic, not a result
that can be caught with this pattern. See the standard-library return types.

## List/map access fails

Lists/text use integer indexes starting at zero. `-1` is the last item. Maps use
text keys. Index assignment does not extend a list. Reading a missing map key
fails; use `contains(map, key)` first. Text is immutable.

```indium error
items = [49]
print(items[1])
```

Use `push(items, value)` to append. A range is lazy and cannot be indexed even
though its coarse runtime type is `list`. There is no `items[1:3]` slice syntax.

## The window never opens or drawing fails

Set `mode = 2d` before scheduling. Let top-level initialization finish: a
top-level infinite loop prevents the runtime from opening its window. Put
ongoing logic in `every tick`/`every frame` handlers.

Drawing during initialization fails:

```indium error
mode = 2d
renderer.clear("black")
```

Move drawing into `every frame { ... } loop`. Input polling also requires the
initialized window, so put it in handlers. `every frame` without `mode = 2d`
is an error. A terminal-only executable must be rebuilt with default features
to gain graphics/audio support.

An OpenGL/window creation failure may be a display, SDL, or driver problem.
Check the backend message. The implementation explicitly requests OpenGL and
does not silently switch to SDL's software renderer. `mode = 3d` is recognized
but deliberately reports that it is not implemented.

## Shapes, text, images, or input look wrong

Clear once at the beginning of each frame, then draw in the order you want.
Later drawing covers earlier drawing. Check X/Y positions and positive sizes.
Color channels must be integer 0..255, and color names are lowercase.

For colored text, use `renderer.text("label", x, y, 24, "red")`. Passing the
color as the fourth argument puts text where a numeric font size belongs.
`renderer.image` takes either 3 or 5 arguments. Use PNG/JPEG/BMP and verify the
source-relative asset path. Cached images do not automatically reload when a
file changes during the same run.

Use `key_down` for held movement and `key_pressed` for a one-time action. Focus
loss clears input state. Tick/frame edge buffers are separate, but several
catch-up ticks do not each count as a new press.

## There is no sound

Check that the build includes media, the file is OGG/Vorbis or PCM WAV, the path
is correct, the sound is mono/stereo, volume is above zero, and the playback
device opens. MP3 and Opus-in-OGG are not supported by this snapshot.

A terminal program that ends immediately also stops its sound immediately.
Use a short sleep in a terminal demo or keep a scheduler running. A paused
voice reports `audio.playing(handle) == false`; that is expected. Completed
handles also report false. There is no `audio.stop()` with zero arguments.

Dummy-device testing verifies state/decoding but produces no audible sound.
If the `SDL_AUDIODRIVER=dummy` test environment variable is set on your own
machine, remove it before testing speakers.

## Simulation or timers seem slow

TPS/FPS are requested rates rather than guaranteed throughput. Handlers execute
on one thread. `sleep`, terminal input, HTTP, large file work, and heavy drawing
delay other work. Use `game.dt` for fixed-tick movement and `time.dt` only when
you intentionally want the last render-frame duration.

Inspect `game.dropped_ticks` if the catch-up limit is discarding due ticks.
Timers skip missed invocations rather than firing every overdue occurrence.
See [Scheduling](scheduling.md). `--frames` does not bound terminal-only timers;
use a stop condition or `--max-steps` as appropriate.

