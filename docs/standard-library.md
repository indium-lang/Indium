# Standard-library reference

Unless stated otherwise, invalid argument types/counts produce source-location
diagnostics. `none` is the normal return value for actions without a value.
All filesystem and asset paths resolve relative to the executing source file.

`?` marks an optional **positional** argument in the signatures below. It is
reference notation, not syntax. A return value of `none` means an action
completed without producing data. Runtime diagnostics abort execution; explicit
`success(...)`/`error(...)` results are ordinary values your code can inspect.

## What needs an import

| Names | How to access them |
| --- | --- |
| General helpers, scalar conversions, collection/text helpers, `success`, `error` | Already available |
| `window`, `renderer`, `input`, `audio` | Already available; media support is part of the default build |
| `math`, `files`, `net`, `time`, `game` | Import the namespace first, for example `import math` |
| `text` helper namespace | Available through `text`; `import text` is also accepted |

An alias such as `import files as fs` uses the same APIs as the original name.
Properties use `module.name`; functions add parentheses. For example,
`time.elapsed` is a property and `time.now()` is a function.

## Terminal and general helpers

| API | Returns | Behavior |
| --- | --- | --- |
| `print(values...)` | `none` | Format any number of values, separated by spaces, followed by a newline |
| `write(values...)` | `none` | Same spacing, without the final newline; output is flushed |
| `input(prompt?)` / `input.read(prompt?)` | `text` | Optionally format/print one prompt, then read one line without its line ending |
| `type_of(value)` | `text` | Return the runtime type name |
| `assert(condition, message?)` | `none` | Require a boolean; a false condition raises the provided formatted message or `Assertion failed` |
| `args()` | `list` | Text arguments provided after the CLI `--` separator |
| `stop()` | `none` | End execution normally; takes no exit-code argument |
| `sleep(seconds)` | `none` | Block the runtime thread for a number of seconds in 0..86400 |

`print()` emits a blank line. `write()` with no arguments emits nothing. Lists,
maps, results, and other values are printed in a human-readable representation,
not serialized JSON. Printing a decimal can look like an integer (`1.0` prints
as `1`); use `type_of` or `is decimal` to inspect its type.

`input` accepts zero or one prompt value; prompts need not be text because they
are formatted. An empty line returns `""`. End-of-input also yields empty text,
so a loop should not assume it will always receive another nonempty answer.
Input failures are diagnostics. `input.read` is terminal input, distinct from
the same namespace's keyboard/mouse functions.

```indium run
print("Score:", 49)
write("No newline yet. ")
print("Now there is one.")
assert(49 > 0, "Score must be positive")
print(type_of(49), type_of(49.0)) // integer decimal
```

## Conversions and result constructors

| API | Accepted input | Return value |
| --- | --- | --- |
| `integer(value)` | Integer, decimal, integer text, boolean | Integer; booleans become 1/0 |
| `decimal(value)` | Integer, decimal, numeric text | Finite decimal |
| `text(value)` | Any value | Formatted text |
| `boolean(value)` | Boolean or exactly `"true"`/`"false"` text | Boolean |
| `list(value)` | A value already in the list category, including a range | The same value; does not collect a range |
| `map(value)` | A value already in the map category | The same map-category value |
| `function(value)` | User function or built-in function value | The same function |
| `any(value)` | Any value | The original value unchanged |
| `success(value)` | Any payload | Successful result |
| `error(value)` | Any payload | Failed result |

`integer(decimal)` truncates toward zero and checks the signed 64-bit bounds.
Numeric text conversions trim surrounding whitespace. Boolean text conversion
does not trim or ignore case: `"True"` and `" true "` are invalid. `decimal(true)`
is invalid; `integer(true)` is valid. Invalid conversions raise a diagnostic;
they do not return an error result. `list()` and `map()` with no arguments are
invalid: use `[]` and `{}` to create empty collections.

`any(value)` does not make a new variable dynamically typed; use `name: any = ...`
for that. Collection/function conversions do not clone or transform the value.
There is no callable `none()` constructor; `none` is the literal absence value.

```indium run
print(integer(" 22 "), integer(-2.9), integer(true)) // 22 -2 1
print(decimal("2.5"), boolean("false"))             // 2.5 false
result = success({answer: 49})
print(result.ok, result.value.answer)               // true 49
failure = error("not found")
if failure is error { print(failure.error) }
```

Result properties are `.ok` (always boolean), `.value` (successful payload only),
and `.error` (failed payload only). Wrong-state field access raises a diagnostic.
Type annotations have no separate `result` name; a function can return a result
without annotating its return, or use `-> any`.

## Lists, maps, and text

| API | Returns | Behavior |
| --- | --- | --- |
| `len(value)` | Integer | Number of text scalars, list items, map entries, or range items |
| `push(list, value)` | `none` | Append to the same mutable list |
| `pop(list)` | Removed value or `none` | Remove the last item; empty list returns `none` |
| `keys(map)` | List of text | Sorted keys of an actual map |
| `contains(container, value)` | Boolean | Substring for text, structural value membership for lists, text-key membership for maps |
| `split(text, separator)` | List of text | Split on every occurrence of the literal text separator |
| `join(list, separator)` | Text | Format every list value, then join with the text separator |
| `replace(text, from, to)` | Text | Replace all occurrences of the literal `from` text |
| `trim(text)` | Text | Remove surrounding Unicode whitespace |
| `upper(text)` | Text | Unicode uppercase conversion |
| `lower(text)` | Text | Unicode lowercase conversion |

The text helpers are also available through `text.len`, `text.contains`,
`text.split`, `text.join`, `text.replace`, `text.trim`, `text.upper`, `text.lower`.
Splitting on an empty separator follows Rust's split convention, including an
empty fragment at each end.

For membership in text or a map, the second argument must be text. List
membership compares values structurally. A lazy range is usable with `len`
and `for`, but is not accepted by `push`, `pop`, `join`, or `contains` as a list
container. `keys` expects an actual map rather than a local module view.

`items.length` and `name.length` are property shortcuts for list/text lengths.
Maps and ranges use `len` rather than `.length`. The text namespace aliases have
the same argument rules as the global helpers; for example, `text.len` is the
same implementation as `len`, not a separate method on a text instance.

The operations return new text; the original string is immutable. `split` is
not a regular-expression splitter or automatic whitespace tokenizer. Literal
adjacent separators produce empty fragments; `split("a,,b", ",")` has three
items, with empty text in the middle. `join` accepts mixed list items because it
formats them. No sort, reverse, slice, regex, binary buffer, JSON parser, or
deep-copy helper is built in yet.

```indium run
items = ["one", "two"]
push(items, "three")
print(pop(items), len(items))                  // three 2
record = {z: 3, a: 1}
print(keys(record), contains(record, "a"))     // [a, z] true
print(join([49, true, "items"], " | "))       // 49 | true | items
print(replace("a-b-a", "a", "x"))             // x-b-x
print(upper(trim(" indium ")))               // INDIUM
```

## Math — `import math`

Constants are read-only decimals: `math.pi`, `math.tau` (two pi), and `math.e`.

| Function | Returns | Behavior |
| --- | --- | --- |
| `math.sqrt(x)` | Decimal | Square root; x must not produce an invalid domain |
| `math.abs(x)` | Integer for integer x, otherwise decimal | Absolute value; minimum integer overflows |
| `math.floor(x)` | Decimal | Round down, including toward more-negative values |
| `math.ceil(x)` | Decimal | Round up |
| `math.round(x)` | Decimal | Nearest integer-valued decimal; halfway cases round away from zero |
| `math.sin(x)` | Decimal | Sine, with x in radians |
| `math.cos(x)` | Decimal | Cosine, with x in radians |
| `math.tan(x)` | Decimal | Tangent, with x in radians |
| `math.log(x)` | Decimal | Natural logarithm; a finite valid result is required |
| `math.exp(x)` | Decimal | e raised to x |
| `math.pow(base, exponent)` | Decimal | Floating-point power |
| `math.min(a, b)` | Integer if both inputs are integer, otherwise decimal | Smaller input |
| `math.max(a, b)` | Integer if both inputs are integer, otherwise decimal | Larger input |
| `math.clamp(value, minimum, maximum)` | Decimal | Limit value to ordered bounds; minimum must not exceed maximum |
| `math.random()` | Decimal | Pseudorandom value in `[0, 1)` |
| `math.random_int(minimum, maximum)` | Integer | Integer between both inclusive integer bounds |
| `math.seed(seed)` | `none` | Set the generator from an integer seed; zero uses the fallback seed 49 |

Except where integer-only arguments are stated, math arguments accept integers
or decimals. All results must be finite; invalid domains and overflow raise
diagnostics. `floor`, `ceil`, and `round` return decimals even when printed as
whole numbers; use `integer(...)` if you need an integer index.

The random generator is a simple xorshift, not a cryptographic source. Repeating
a seed and the same sequence of random calls gives repeatable results for this
implementation; avoid treating its exact sequence as a permanent file format.

```indium run
import math
print(math.sqrt(49), math.min(3, 8), math.clamp(30, 0, 5)) // 7 3 5
print(math.floor(-1.2), math.ceil(-1.2), math.round(-2.5)) // -2 -1 -3
print(integer(math.floor(4.9)))                          // 4
math.seed(49)
first = math.random()
math.seed(49)
assert(first == math.random())
print(math.random_int(4, 4))                             // 4
```

## Files — `import files`

| API | Return value |
| --- | --- |
| `files.read(path)` | Result containing UTF-8 text |
| `files.write(path, text)` | Result containing `none`; overwrite/create file |
| `files.append(path, text)` | Result containing `none`; append/create file |
| `files.exists(path)` | Boolean |
| `files.list(path)` | Result containing sorted entry names |
| `files.mkdir(path)` | Result containing `none`; create directories recursively |
| `files.remove(path)` | Result containing `none`; remove one file |

Operational I/O failures return `error(message)`; successful operations return
`success(value)`. There is no recursive deletion API.

Arguments are text paths; `write`/`append` also require text content. A bad
argument type is a diagnostic rather than an I/O result. `read` expects UTF-8
text, not arbitrary binary bytes. `write` overwrites an existing file rather
than appending. `write` and `append` do not create missing parent directories;
use `mkdir` first when needed.

`exists` also returns true for an existing directory. It does not guarantee a
later operation will succeed. `list` lists one directory's entry names in sorted
order, not full paths or a recursive tree. `remove` removes a file, not a
directory tree. Access is governed by normal operating-system permissions.

Example that reads a file if present (checking alone does not perform the read):

```indium
import files
result = files.read("notes.txt")
if result is success {
    print(result.value)
} else {
    print("Could not read notes:", result.error)
}
```

Write with explicit outcome handling:

```indium
import files
written = files.write("notes.txt", "Hello from Indium!\n")
if written is error {
    print("Could not save:", written.error)
    stop()
}
```

The second example creates/overwrites `notes.txt` beside its source file when
executed. Both relative and absolute paths are accepted. An imported function's
path is relative to its defining file, not to the file that called it.

## HTTP — `import net`

`net.get(url)` and `net.post(url, body)` use the installed `curl` program through
argument-based process spawning, without a shell. HTTP(S) only, 15-second timeout,
16 MiB response cap, normal TLS validation, and no automatic redirects.

On transport success, the result contains `{status: integer, body: text}`.
HTTP error status codes such as 404 are still transport successes; inspect
`.status`. Missing curl, TLS/network failures, oversized or non-UTF-8 responses
return an error result. Calls block the runtime thread. The initial POST helper
sends curl's default form content type; custom headers/JSON convenience APIs are
not implemented.

The exact functions are `net.get(url: text)` and `net.post(url: text, body: text)`.
They return success/error results. A success payload has `.status` (integer) and
`.body` (text). No header map or parsed JSON object is returned. A URL with a
non-HTTP(S) scheme is rejected as an argument diagnostic.

On Windows the runtime launches `curl.exe`; a PowerShell alias named `curl`
does not substitute for that executable. You can check it with
`curl.exe --version`. On other systems the executable name is `curl`.

```indium
import net
response = net.get("https://example.com")
if response is error {
    print("Transport failed:", response.error)
} else {
    status = response.value.status
    if status >= 200 and status < 300 {
        print(response.value.body)
    } else {
        print("HTTP status:", status)
    }
}
```

Executing this block accesses the network. It is only checked, not called, by
the documentation verifier. HTTP(S) support does not include a web server,
websockets, custom headers, cookies/session management APIs, streaming downloads,
file-upload helpers, automatic redirects, or a general JSON library.

## Time and game

Use `import time` or `import game` for their namespaces.

| Property/function | Behavior |
| --- | --- |
| `time.now()` / `time.elapsed` | Monotonic seconds since scheduler start; top-level uses initialization time |
| `time.dt` | Seconds since the previous rendered frame |
| `time.sleep(seconds)` | Alias for blocking `sleep` |
| `game.tps` | Simulation ticks/second, default 240; set between 1 and 10000 |
| `game.dt` | Fixed seconds per tick |
| `game.ticks` | Number of dispatched ticks |
| `game.frames` | Number of completed frame iterations; zero during the first frame |
| `game.max_catchup` | Maximum ticks per iteration, default 8; set between 1 and 1000 |
| `game.dropped_ticks` | Tick intervals discarded after overload |
| `game.stop()` | Alias for `stop()` |

A tick counter advances only when tick handlers are registered. Frame handlers
require 2D mode. See the language guide for scheduler semantics.

Only `game.tps` and `game.max_catchup` are writable game properties. `game.tps`
accepts a number and reads back as a decimal; `.max_catchup` requires an integer.
The other counters/deltas and time properties are read-only. `time.now()` and
`time.elapsed` are monotonic relative times, not a calendar date or timestamp.
Their clock resets before the scheduler starts; see [Scheduling](scheduling.md).

## Window — available without imports

Set properties during top-level initialization; drawing begins in frame
handlers after window creation. These properties can also change at runtime:

| Property | Default / accepted values |
| --- | --- |
| `window.title` | `"Indium"`, text |
| `window.width` / `.height` | 800 / 600, integers 1..16384 |
| `window.fps` | 60; 0 means uncapped; maximum 10000 |
| `window.vsync` | `true`; actual support depends on the driver |
| `window.fullscreen` | `false`; enabling uses borderless desktop fullscreen |
| `window.exclusive` | `false`; select exclusive fullscreen when supported |
| `window.backend` | Read-only; `"opengl"` after initialization |

`window.toggle_fullscreen()` toggles `.fullscreen`; `window.close()` exits.
Toggling fullscreen from an existing window preserves its windowed dimensions
for restoration. Resizing a desktop
window updates its reported dimensions. Borderless fullscreen uses the display
size. Exclusive fullscreen selection can fail if the platform rejects it.
There is no monitor-selection or refresh-rate/resolution-selection API yet.

The renderer requests SDL's OpenGL backend explicitly; it does not silently
fall back to a software SDL renderer. Headless Mesa can still supply a software
OpenGL implementation. For SDL's offscreen driver, resizing recreates its window
because that driver does not resize its EGL pbuffer.

## Renderer — built in

| Call | Arguments |
| --- | --- |
| `renderer.clear(color)` | Clear the frame |
| `renderer.rect(x, y, width, height, color?)` | Filled rectangle |
| `renderer.outline(x, y, width, height, color?)` | One-pixel rectangle outline |
| `renderer.line(x1, y1, x2, y2, color?)` | One-pixel line |
| `renderer.circle(x, y, radius, color?)` | Filled circle |
| `renderer.text(text, x, y, size?, color?)` | DejaVu Sans; default size 24 |
| `renderer.image(path, x, y)` | Image at its original size |
| `renderer.image(path, x, y, width, height)` | Scaled image |
| `renderer.save(path)` | Capture the current frame, e.g. to a `.png` file |

Draw inside `every frame { ... } loop`. The runtime presents the frame after
handlers return. Coordinates start at the top-left. Colors accept `#RRGGBB`,
`#RRGGBBAA`, `[r, g, b]`, or `[r, g, b, a]` with integer channels 0..255.
Names: `black`, `white`, `red`, `green`, `blue`, `indium`. The default drawing color
is white. Color name `indium` is a temporary convenience color, not final branding.

PNG, JPEG, and BMP decoding is enabled. Source images are bounded to 8192 by
8192 with a 128 MiB decode allocation limit. Font size is 1..256; individual text
runs are bounded. Decoded images and text rasters are cached in CPU memory;
GPU textures are currently uploaded per draw and released immediately. This is
a functional initial implementation with room for batching/persistent GPU caches.

Every renderer call returns `none` on success and raises a diagnostic on a
failure. `clear` requires a color; other optional colors default to white.
For text, supplying a color also requires supplying the font-size argument first.
Image calls accept exactly 3 or 5 arguments, not 4. Dimensions/radii must be
positive numbers in 1..16384; coordinates accept numbers in -1000000..1000000
and truncate to integer pixels. Window dimensions and FPS, in contrast, require
actual integers. See [Graphics, input, and audio](graphics-and-audio.md) for
complete programs, asset handling, screenshots, caching, and input examples.

## Keyboard and mouse — built in

- `input.key_down("space")`: currently held.
- `input.key_pressed("space")`: newly pressed since the last tick/frame.
- `input.mouse_down("left")`, `input.mouse_pressed("left")`.
- `input.mouse_x`, `input.mouse_y`: latest window coordinates.

Key names use SDL key names, e.g. `left`, `right`, `escape`, `space`, `F11`.
Mouse buttons: `left`, `right`, `middle`. Unknown names raise diagnostics.
Key repeat events do not create repeated press edges. Tick and frame handlers
have independent press-edge buffers so a slow render frame does not consume
an edge before simulation sees it. The edge lasts for one tick and one frame;
multiple handlers in that same phase can see it. Held keys clear on focus loss.

## Audio — built in

```indium
sound = audio("assets/tone.ogg")
// Optional positional arguments: volume, then looping.
music = audio.play("assets/tone.ogg", 0.5, true)
audio.pause(music)
audio.resume(music)
audio.volume(music, 0.25)
audio.stop(music)
print(audio.playing(music))
```

Actual syntax is positional; named arguments in the comment above describe the
defaults, not supported call syntax. `audio` and `audio.play` return a positive
integer voice handle. Volume is 0..1. `playing` is false for paused, completed,
or stopped handles. Other control functions report whether a live handle was
found. Up to 128 simultaneous voices are mixed and clipped into stereo output.

Implemented formats: OGG with Vorbis, and PCM WAV; mono/stereo input. Mixing
resamples source rates to the opened device's rate. MP3, Opus-in-OGG, streaming,
and multichannel audio are not implemented. Files are decoded once and cached
in memory, with a 256 MiB limit per decoded sound. Short effects and moderate
music tracks work; this is not yet a streaming music engine.

Audio initialization is lazy, so a terminal program does not open a sound device
unless it calls the audio API. Device/format failures are source diagnostics.
Returning from a program stops its audio; use a scheduler or sleep to keep a
terminal audio program alive long enough for playback.
