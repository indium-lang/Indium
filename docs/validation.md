# Validation for 0.1.0-alpha.1

Development snapshot: 2026-10-29. The project was implemented and tested locally.
No GitHub repository changes, pushes, or external publishing were performed.

## Environment

- Linux x86-64, Ubuntu 24.04 userspace.
- Rust/Cargo 1.99.1.
- SDL2 2.30, OpenGL through SDL's explicit `opengl` renderer.
- Mesa software OpenGL with `SDL_VIDEODRIVER=offscreen` for graphical checks.
- `SDL_AUDIODRIVER=dummy` for playback-state smoke tests.

The environment had the SDL2 runtime but no development linker alias. Local
build/test commands added `RUSTFLAGS='-L native=/tmp/indium-native/lib'` pointing
to an SDL2 linker alias. This environment-specific path is not required on a
normal system with SDL2 development libraries installed and is not a project
configuration setting.

## Automated checks actually completed

| Check | Result |
| --- | --- |
| `cargo test --locked` with default media features | 59 passed; one display-dependent test intentionally excluded from this invocation |
| `cargo test --no-default-features --locked` | 57 passed |
| Opt-in OpenGL test, explicitly run with offscreen Mesa | 1 passed |
| `cargo fmt --check` | Passed |
| `cargo clippy --all-targets --locked -- -D warnings` | Passed |
| `cargo build --release --locked` | Passed |
| `indium check` on every shipped example/fixture `.in` file | 12 source files checked |
| Packaged Linux launcher `--version` and `run examples/hello.in` | Passed |

There are **60 distinct automated tests** exercised across the default suite and
its opt-in graphics test. The terminal-only run repeats applicable tests; it is
not 57 additional distinct tests.

Coverage includes lexical tokens/locations, malformed input, precedence,
inference/annotations, arithmetic errors, booleans, conditionals, lexical scopes,
functions/closures, recursion guards, loop control, collections, single evaluation
of assignment targets, Unicode, results, imports/cycles/cached mutable module
state, filesystem operations, real local HTTP GET/POST requests, CLI failures,
timer/tick execution, and simulated 240 TPS at 30/60/144/240/360 rendering FPS.

Audio tests decode the actual included OGG/Vorbis and PCM WAV files, verify their
sample rate/duration/energy, and exercise a mixer with concurrent, looping,
paused, completed, and clipped voices. The SDL dummy-device smoke program checks
play/pause/resume/volume/stop/playing for both formats. It does **not** establish
that speakers emitted correct audible sound.

The OpenGL test checks the selected backend, injected key events, independent
tick/frame press edges, resize state, borderless fullscreen state transitions,
and framebuffer pixel colors. Fullscreen passed in this offscreen environment;
exclusive fullscreen and monitor-specific behavior remain unverified.

## Programs executed successfully

| Source | Verified behavior |
| --- | --- |
| `examples/hello.in` | Greeting and range sum of 55 |
| `examples/factorial.in` | Values 0! through 20! |
| `examples/imports.in` | Imported geometry functions, including 4 × 6 = 24 |
| `examples/collections.in` | Mutable lists/maps, iteration, text helpers |
| `examples/files.in` | Write/read a real text file; generated file then removed |
| `examples/timer.in` | Three one-second recurring invocations, then stop |
| `examples/audio.in` | OGG and WAV playback states using SDL's dummy device |
| `examples/2d.in` | Twelve real OpenGL frames with shapes, image, text, tick updates |
| `tests/fixtures/render.in` | Rendered PNG inspected visually and checked for exact pixels |
| `tests/fixtures/fullscreen.in` | Resize/fullscreen transitions; final 400 × 300 PNG and pixel colors checked |

`examples/http.in` was checked but its public internet request was not executed.
Networking execution was verified against a local test server with actual curl
GET and POST subprocesses. The 2D demo's physical keyboard/speaker interactions
were not manually exercised; keyboard state was verified with injected SDL events.


## Windows setup update

`scripts/test-windows-setup.ps1` exercises real filesystem installation,
reinstallation, upgrades, source validation, uninstall preservation, marker and
symlink protection, and long/quoted/case-varied PATH transformations. It also
executes the build helper with a mocked Cargo command to check SDL paths with
spaces, argument preservation, DLL copying, and error cleanup. Native registry,
environment notification, and executable version calls are mocked explicitly.

All **35 setup checks passed** under PowerShell 7.6.9 on Linux. They do not establish that the
Windows file picker, user registry writes, Installed Apps entry, or broadcast
works on a native Windows desktop. The new installer needs that final smoke test.

## Documentation update

The expanded handbook was checked against the shipped interpreter and source
API inventory, without changing the Rust language implementation.

| Documentation check | Result |
| --- | --- |
| Markdown files examined, including project README | 13 |
| Callable names covered, including aliases/conversions | 88 |
| Namespace properties covered | 21 |
| Local Markdown links checked | 43 |
| Complete Indium code blocks verified | 45 |
| Valid code blocks executed | 26, including two three-frame offscreen graphics programs |
| Deliberately failing examples verified to fail | 5 |
| Explicitly labeled partial fragments | 5; excluded from standalone checking |
| Shipped example files checked | 14 |
| New tutorial programs executed | 4; the guessing game received controlled stdin |
| Offline HTML book | 14 chapters; generated internal anchors checked |

The documentation checker runs source snippets in temporary files and copies
example fixtures into a disposable directory. Filesystem-writing and public
HTTP snippets are checked rather than executed. These checks do not add physical
keyboard, speaker, GPU-driver, or Windows-installer verification.
The offline handbook's structure and links were validated. A browser visual
pass could not be completed because the environment's browser download was
unavailable; the layout is not claimed to have been visually verified here.
