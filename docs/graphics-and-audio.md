# Graphics, input, and audio

The default Indium build includes OpenGL-backed 2D rendering, keyboard/mouse
input, and OGG/WAV audio. `window`, `renderer`, `input`, and `audio` are available
without imports. A terminal-only build cannot use the media backend.

## Your first window

Save this as `drawing.in`:

```indium frames=3
mode = 2d
window.title = "My first Indium window"
window.width = 640
window.height = 400
window.vsync = false

every frame {
    renderer.clear("#141724")
    renderer.rect(40, 70, 160, 90, "indium")
    renderer.circle(360, 120, 45, "#58d6a7")
    renderer.line(40, 230, 500, 230, "white")
    renderer.text("Hello, Indium!", 40, 280, 28, "white")
} loop
```

Run it with `indium run drawing.in`. The window stays open until closed. The
three-frame limit shown in the documentation test marker is only a verifier
setting; it is not part of the program's runtime behavior.

Top-level code configures the window. It opens after initialization finishes.
Draw inside `every frame`, after that point. The runtime presents the completed
frame automatically; there is no user-callable `present` function.

Use one `renderer.clear` at the beginning of each frame. Drawing order matters:
later shapes and images are drawn over earlier ones. There is no built-in scene
graph, sprite object, camera transform, layer system, or collision engine.

## Coordinates, shapes, and colors

The origin `(0, 0)` is the top-left. X increases to the right and Y downwards.
Rectangles/images use a top-left corner plus width and height. A circle's X/Y
position is its center. Lines use two endpoint positions.

Coordinates accept integers or decimals and are converted to integer pixels
by truncation toward zero. Allowed coordinates are -1000000 through 1000000.
Widths, heights, and circle radii accept numeric values in 1..16384, then are
truncated. Keep positive sizes even when a shape is partly offscreen.

Colors accept:

| Form | Example | Meaning |
| --- | --- | --- |
| RGB hex text | `"#7163ff"` | Opaque red/green/blue channels |
| RGBA hex text | `"#7163ff80"` | RGB plus alpha at the end |
| RGB list | `[113, 99, 255]` | Three integer channels, each 0..255 |
| RGBA list | `[113, 99, 255, 128]` | Three channels plus alpha, each 0..255 |
| Named color | `"white"` | `black`, `white`, `red`, `green`, `blue`, or `indium` |

Alpha 0 is transparent and 255 is opaque. Omitted optional drawing colors are
white. Named colors are lowercase. `indium` currently means `#7163ff`; it is a
temporary convenience color rather than a finalized brand palette.

`renderer.rect` is filled, `renderer.outline` is a one-pixel rectangle border,
`renderer.line` is one pixel wide, and `renderer.circle` draws a filled circle.
There are no rotation, gradients, stroke-width, polygon, or shader APIs yet.

## Text and images

Fragments to place inside a frame handler:

```indium fragment
renderer.text("Default size is 24", 20, 20)
renderer.text("Larger text", 20, 60, 36, "#ffffff")
renderer.image("assets/player.png", 20, 130)
renderer.image("assets/player.png", 100, 130, 64, 64)
```

Text uses the embedded DejaVu Sans font. It supports Unicode text within that
font's glyph coverage; no custom-font loader is exposed. Font size accepts a
number from 1 through 256. To set a text color, supply the size first, followed
by the color. Text runs are limited to 16384 UTF-8 bytes and a raster size of
16384 by 4096 pixels. An empty text run draws nothing.

Images support PNG, JPEG, and BMP, including source alpha where the format
supports it. The three-argument image call uses the original dimensions. The
five-argument form scales to the requested width/height. There is no
four-argument form, source-rectangle cropping, image tint parameter, or rotation.
Decoded source images are limited to 8192 by 8192 and a 128 MiB decode allocation.

All asset paths are relative to the `.in` file containing the call (or the
function's defining file). Use the supplied `examples/assets/checker.png` when
trying the existing `examples/2d.in`; the `player.png` fragment above assumes
you supply that file yourself.

Decoded images are cached by canonical path. Replacing the file during a run
does not automatically reload the cached image. Text rasters are cached by text
and size, with a small cache that clears after 128 entries. GPU textures are
currently uploaded per draw and immediately released. This works, but is not
yet optimized batching or persistent sprite storage. Avoid repeatedly drawing
huge images or constantly generating large unique text strings.

## Screenshots

Call `renderer.save("capture.png")` after drawing everything you want to capture,
inside the frame handler. It reads the current framebuffer and writes an image
relative to the calling source file. The parent directory must already exist.
Use a `.png` suffix for PNG output. A failed save is a runtime diagnostic, not a
success/error result.

For a one-time capture, use a persistent flag declared before the handler:

```indium fragment
// Top-level: saved = false
if not saved {
    renderer.save("capture.png")
    saved = true
}
```

## Held keys versus new presses

| Call | When it is true | Typical use |
| --- | --- | --- |
| `input.key_down("right")` | The key is currently held | Continuous movement |
| `input.key_pressed("space")` | A new press occurred since the last matching phase | Jump, menu action, sound |
| `input.mouse_down("left")` | The mouse button is held | Dragging, continuous drawing |
| `input.mouse_pressed("left")` | A new button press occurred | Click action |

Input requires an initialized 2D window. Polling input during top-level setup
fails because the window has not opened. Key names use SDL's key names, such as
`left`, `right`, `up`, `down`, `space`, `escape`, and `F11`. Unknown names fail.
Mouse button names are exactly `left`, `middle`, and `right`.

`input.mouse_x` and `input.mouse_y` give the most recently received window mouse
coordinates. They start at zero. There is no mouse wheel, text-entry event,
gamepad, raw relative mouse, or configurable input-binding API yet.

Key-repeat events do not create extra new presses. Tick and frame handlers have
separate press-edge buffers, so input seen by rendering is not consumed before
simulation sees it. Multiple handlers within one phase can see the same edge.
Focus loss clears held/pressed states.

## Movement with a fixed tick

```indium frames=3
mode = 2d
import game
import math
window.title = "Move with arrows"
window.width = 640
window.height = 400
window.fps = 60
window.vsync = false
game.tps = 240
x = 100.0
y = 100.0
speed = 180.0

every tick {
    if input.key_down("left") { x -= speed * game.dt }
    if input.key_down("right") { x += speed * game.dt }
    if input.key_down("up") { y -= speed * game.dt }
    if input.key_down("down") { y += speed * game.dt }
    x = math.clamp(x, 0, window.width - 30)
    y = math.clamp(y, 0, window.height - 30)
    if input.key_pressed("F11") { window.toggle_fullscreen() }
    if input.key_pressed("escape") { stop() }
} loop

every frame {
    renderer.clear("#141724")
    renderer.rect(x, y, 30, 30, "indium")
    renderer.text("Arrows move; F11 fullscreen; Escape quits", 12, 12, 18)
} loop
```

The speed is pixels per second. `game.dt` turns it into pixels per simulation
tick. X and Y start as decimals because movement produces decimal values. The
frame handler only draws the latest state. See [Scheduling](scheduling.md) for
the difference between `game.dt` and the render delta `time.dt`.

## Window settings and fullscreen

`window.title`, `.width`, `.height`, `.fps`, `.vsync`, `.fullscreen`, and
`.exclusive` can be configured before opening and changed while running.
`window.backend` is read-only. It is `"not initialized"` before a normal media
window opens and `"opengl"` afterward; terminal-only builds report `"unavailable"`.

To start in borderless fullscreen, set `window.fullscreen = true` at top level.
The default `window.exclusive = false` uses desktop fullscreen. For exclusive
fullscreen, set `.exclusive = true` as well; the platform can reject it. There
is no monitor selector, display-mode selector, or refresh-rate selector yet.

`window.toggle_fullscreen()` flips the fullscreen setting. Toggling from an
existing window preserves the prior windowed size for restoration. Native resize events update the reported
window dimensions. Borderless fullscreen uses the display size. Width/height
configuration is mainly meaningful in windowed mode.

`window.fps` is a software render cap. VSync may cap it further, and a slow
machine may achieve less. For a high requested FPS or uncapped rendering,
disable VSync explicitly if appropriate. Even then, nothing guarantees the
requested rate. The backend explicitly requests OpenGL; it does not switch to
SDL's software renderer when OpenGL fails.

## Playing sound

The shortest call is `audio("sound.ogg")`. The equivalent explicit call is
`audio.play("sound.ogg")`. Both return a positive integer handle for one voice.

```indium fragment
// Assumes assets/music.ogg exists beside this program's assets folder.
music = audio.play("assets/music.ogg", 0.5, true)
audio.pause(music)
audio.resume(music)
audio.volume(music, 0.25)
print(audio.playing(music))
audio.stop(music)
```

The optional arguments are volume (default 1.0, range 0..1) and looping (default
false). They are positional. Looping requires the third argument, so include a
volume value even if you want the default volume.

Pause/resume/stop/volume return a boolean indicating whether a live handle was
found. `audio.playing(handle)` returns false for paused, finished, or stopped
voices. `audio.volume` sets the volume; it is not a getter. Reusing a file starts
a separate voice, so effects can overlap. At most 128 voices coexist, including
paused voices. The mixer clamps combined output into the stereo sample range.

Audio supports OGG with Vorbis and PCM WAV, with mono or stereo source audio.
The runtime resamples to the opened device's rate and mixes stereo output. It
does not support MP3, Opus-in-OGG, multichannel sources, streaming, seeking,
pitch changes, device selection, or a separate master-volume control.

Files are decoded once and cached by canonical path. Each decoded sound is
limited to 256 MiB; it is not streamed from disk. Sound devices open lazily on
the first playback call, including in terminal/background programs. File,
codec, or device failures are runtime diagnostics rather than result values.

## Keep a terminal sound alive

When a terminal script ends, its audio stops too. The supplied `examples/audio.in`
uses brief sleeps to demonstrate playback before exiting. Another approach is
a timer that keeps the program alive until the sound finishes. Do not block a
2D handler with a sleep to wait for music; let the scheduler continue.

There is no `audio.stop()` with no arguments to stop all voices. Call
`audio.stop(handle)` for a particular sound. The global `stop()` function exits
the entire program.

## What has been verified

OpenGL rendering and synthetic input were tested with an offscreen Mesa driver;
audio decoding/mixing and playback state were tested with a dummy device. The
user has confirmed the Windows terminal build and hello program work. Physical
Windows graphics, real speakers, monitor-specific VSync, and exclusive
fullscreen still need testing. See the [validation record](validation.md).
