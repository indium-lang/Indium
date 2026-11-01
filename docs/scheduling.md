# Ticks, frames, and timers

Indium can execute a normal terminal script, keep a timer-driven program alive,
or run a 2D window with independent simulation and rendering rates. All Indium
handlers execute on the same runtime thread.

## Program lifecycle

1. The loader reads, parses, and checks the entry file and local import graph.
2. Top-level statements run in order. They initialize variables, import modules,
   configure the window, and register recurring handlers.
3. If `stop()` was called during initialization, execution ends immediately.
4. Otherwise, `2d` mode opens the window; terminal/background mode opens none.
5. The scheduler runs registered ticks/timers and, for 2D, graphical frames.

A terminal/background file without handlers finishes after its statements run.
A 2D program keeps polling its window even if no handlers were registered.
An endless top-level loop prevents initialization from finishing, so a 2D window
will never open. Put ongoing work in recurring handlers instead.

Functions are hoisted within their scope, but variables initialize in source
order. Declare a handler's captured variables before declaring that handler.
Handlers must be at module level, outside functions, loops, conditions, or other
handlers. Imported modules may also declare module-level handlers.

## Three handler forms

| Form | Trigger | Typical work |
| --- | --- | --- |
| `every tick { ... } loop` | Fixed simulation step | Movement, physics, game rules |
| `every frame { ... } loop` | Render deadline, in 2D mode | Clear and draw the next frame |
| `every 500 milliseconds { ... } loop` | Recurring real-time interval | Status messages, periodic updates |

The trailing `loop` is required in every form. `second`/`seconds` and
`millisecond`/`milliseconds` are accepted. There is no `every minute` form; write
`every 60 seconds` instead.

```indium run
count = 0

every 10 milliseconds {
    count += 1
    print("Timer:", count)
    if count == 3 {
        stop()
    }
} loop
```

This prints three timer messages, then exits. The interval expression is
evaluated once when the handler is registered. Changing the variable used to
calculate it later does not change the registered interval. Supported intervals
are 0.000001 seconds through 365 days; actual precision depends on the runtime
and operating system. A one-microsecond request is not a real-time guarantee.

Each invocation gets a fresh block scope. Variables created only inside that
block do not persist to the next invocation. Assign to an existing outer
variable, such as `count`, to retain state.

`break` and `continue` apply to an actual `for`, `while`, or `loop` inside the
handler. They cannot terminate the recurring registration itself. Use `stop()`
to finish the program; individual handlers cannot currently be unregistered.

## Simulation TPS and fixed delta time

```indium run
import game
game.tps = 100
position = 0.0

every tick {
    position += 50 * game.dt
    if game.ticks == 3 {
        print("Three ticks completed.")
        stop()
    }
} loop
```

`game.tps` means ticks per second. Its default is 240, and it accepts numeric
values from 1 through 10000. `game.dt` is the fixed duration of the current tick
in seconds: `1 / game.tps`. Multiplying a speed in pixels/second by this delta
makes a movement update express a distance per tick.

The scheduler increments `game.ticks` before running that tick's handlers. Ticks
are only dispatched when at least one tick handler exists; simply importing
`game` does not start a simulation counter.

## Rendering FPS and frame delta time

`window.fps` requests a maximum render frequency: default 60, or 0 for uncapped.
`time.dt` is the actual monotonic time between frame starts, not `1 / fps`.
`window.vsync` can further restrict presentation to the monitor/driver cadence.

Rendering FPS does not determine TPS. A 240-TPS simulation can request 60, 144,
or another rendering FPS. These are requested schedules, not performance
guarantees. There is no automatic interpolation between previous/current tick
states in this release; a frame draws whatever state your handlers have produced.

`game.frames` counts completed frame iterations. It is zero while the first
frame's handlers run. `time.dt` starts at 0 before scheduling and can be very
small on the first frame. It is a render value, so do not use it as a timer or
fixed-tick delta in a terminal-only program.

`time.now()` and `time.elapsed` both report monotonic elapsed seconds. During
top-level initialization they measure from interpreter creation. The start
instant is reset just before the scheduler loop; a timestamp taken during
initialization is therefore not a persistent wall-clock timestamp. No calendar
date, time zone, or system-clock API is exposed yet.

## What happens during one scheduler pass

| Order | Action |
| --- | --- |
| 1 | Poll window events in 2D mode |
| 2 | Accumulate elapsed time and dispatch due simulation ticks |
| 3 | Dispatch due timers |
| 4 | If a frame is due, run frame handlers and present it |
| 5 | Sleep until the nearest relevant deadline, subject to a small polling cap |

Multiple handlers in the same phase run in registration order. All handlers
for one frame draw onto the same backbuffer, which is presented after they
finish. If multiple handlers clear it, later clears erase earlier drawing.

Keyboard/mouse press edges have separate tick and frame buffers. A press is
visible to all handlers in the next tick and all handlers in the next frame;
it is then cleared for that phase. Several catch-up ticks do not each see a new
press. See [Graphics, input, and audio](graphics-and-audio.md).

## Catch-up, overload, and missed timers

If the runtime falls behind, the simulation can process several ticks in one
scheduler pass. `game.max_catchup` defaults to 8, accepts integer values from 1
through 1000, and bounds that burst of work. Excess due tick intervals are
discarded and counted in `game.dropped_ticks`.

An overdue timer fires once, then advances its next deadline to the next point
on its original cadence. It skips missed invocations rather than replaying
all of them in a burst.

Increasing the catch-up limit can help preserve tick counts under a blocking
renderer, but does not make slow handler code faster. Very large bursts can
also delay rendering and input processing. Measure the program and reduce
blocking or expensive work before relying on extreme limits.

## Blocking and stopping

`sleep`, terminal input, filesystem work, HTTP requests, and drawing execute
synchronously on the runtime thread. A long call delays ticks, timers, and
frames. `sleep` is useful in a simple terminal audio demo; using it to pace a
2D handler blocks the entire runtime.

The audio callback mixes playback on its internal device thread after a sound
has been started. This does not make Indium handler code multithreaded.

`stop()`, `game.stop()`, and `window.close()` end the whole program normally.
Closing the native window also ends it. A tick/frame/timer handler that calls
one of these should not rely on later statements running: the interpreter
checks the stop state between statements.

For short graphics tests, use `indium run drawing.in --frames 3`. To bound
interpreter work, use `--max-steps`; neither option is a general wall-clock
timeout. See the [CLI reference](cli.md).
