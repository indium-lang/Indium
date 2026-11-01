# Indium handbook

Documentation for **0.1.0-alpha.1**, covering the implementation shipped in this
project. Examples use real Indium syntax. Proposed features are marked as
unfinished rather than presented as working APIs.

If you downloaded the release ZIP, open **docs/guide.html** for this entire
handbook in one offline page. Use your browser's Find command to search it.

## Start here

| Chapter | What you will learn |
| --- | --- |
| [Getting started](getting-started.md) | Install the command, create a file, run your first program, and understand folders |
| [Language](language.md) | Values, types, operators, scopes, functions, collections, results, and imports |
| [Standard library](standard-library.md) | Every built-in function, module, property, argument, and return value |
| [Graphics, input, and audio](graphics-and-audio.md) | Create a window, draw, respond to controls, use fullscreen, and play sound |
| [Ticks, frames, and timers](scheduling.md) | Understand the program lifecycle, TPS/FPS, delta time, and recurring handlers |
| [Command-line reference](cli.md) | Commands, flags, program arguments, diagnostics, and execution limits |
| [Recipes](recipes.md) | Small programs and patterns you can adapt |
| [Troubleshooting](troubleshooting.md) | Fix common installation, syntax, type, graphics, and audio problems |
| [Windows installation](windows-installation.md) | Installer details, updates, removal, and source-build requirements |
| [Implementation map](architecture.md) | How the Rust source is organized and how execution works |
| [Validation record](validation.md) | What has actually been tested and what still needs verification |

New to programming? Read Getting started, then the Language chapter in order.
Already comfortable with programming? Start with the Language and Standard
library chapters, then use the others when you need them.

## What this release can do

| Area | Current capability |
| --- | --- |
| Execute source | Run UTF-8 `.in` files through a Rust interpreter |
| General programming | Variables, scalar type checks, arithmetic, branches, loops, functions, closures, and recursion |
| Data | Mutable lists/maps, text, lazy integer ranges, and success/error results |
| Modules | Built-in imports, relative local imports, aliases, cached module state |
| Terminal | Output, line input, command-line arguments, diagnostics |
| Files | Read, write, append, list, create directories, and remove files |
| Network | Blocking HTTP(S) GET/POST through an installed curl executable |
| 2D graphics | OpenGL window, shapes, images, text, screenshots, keyboard/mouse input |
| Audio | Cached OGG/Vorbis and PCM WAV, volume, looping, pause/resume, multiple voices |
| Timing | Independent fixed simulation ticks, rendering frames, recurring timers |
| Windows setup | Install an already-built runtime for your account and add it to PATH |

The language runtime is included in one executable plus SDL2's runtime library.
Graphics and audio are available without an Indium package download or import.
The first release is still a pre-alpha: expect limitations and further testing.


## Reading the examples

`name?` in a reference signature means an optional positional argument. It is
documentation notation, not something you type. Function calls use positional
arguments; there are no named arguments or default parameters on user functions.

Examples labeled **fragment** need the surrounding program described nearby.
Examples labeled **error** deliberately show code that fails. Other Indium code
blocks are complete programs checked by the documentation verifier. Blocks
marked `run` are also executed; graphics blocks marked `frames=3` run for three
offscreen frames during documentation checks.

Shell commands are separate from Indium source. Do not copy a terminal prompt
such as `PS C:\Users\v>` into either kind of code.

## Shipped programs

| File inside `examples/` | Purpose |
| --- | --- |
| `hello.in` | Greeting, typed function, and a range sum |
| `factorial.in` | Function calls and factorials through 20! |
| `imports.in` / `modules/geometry.in` | Local module exports and built-in math |
| `collections.in` | Lists, maps, iteration, and text helpers |
| `arguments.in` | Read values after the CLI's `--` separator |
| `closures.in` | Return a function that keeps private mutable state |
| `results.in` | Return and inspect explicit success/error results |
| `guessing-game.in` | Interactive game with validated numeric input |
| `files.in` | Write and read a source-relative text file; this creates `indium-note.txt` |
| `timer.in` | Run a recurring timer three times, then stop |
| `2d.in` | Moving ball, shapes, image, text, arrow keys, Space sound, F11, Escape |
| `audio.in` | OGG/WAV playback controls |
| `http.in` | Make an HTTP request and inspect its result; requires network access |

The asset files supplied with these demos are described in
`examples/assets/README.md`. Keep the example folder structure intact so relative
imports and asset paths continue to work.
