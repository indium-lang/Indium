# Recipes

These patterns use the current language and built-in library. Code blocks
marked `run` are complete examples with no interactive input or external
network requests. File-writing examples say what they create.

## Count occurrences with a map

```indium run
words = split(lower("red blue red green blue red"), " ")
counts = {}
for word in words {
    if contains(counts, word) {
        counts[word] += 1
    } else {
        counts[word] = 1
    }
}
for word in counts {
    print(word, counts[word])
}
```

The output order is `blue 2`, `green 1`, `red 3`, because maps iterate sorted
keys. Reading a missing key would fail, so the code checks `contains` first.
This example splits on a literal space; real text with punctuation or multiple
whitespace kinds needs your own normalization/tokenization rules.

## Filter and reverse a list

```indium run
values = [5, 12, 3, 18, 9]
large = []
for value in values {
    if value >= 10 { push(large, value) }
}

reversed = []
for offset in 0..len(large) {
    push(reversed, large[-1 - offset])
}
print(large)    // [12, 18]
print(reversed) // [18, 12]
```

This creates new outer lists. There is no built-in filter/reverse API, slicing,
or list comprehension yet. Iterating a list uses a snapshot of its elements, so
adding items to the original does not extend an already-running `for` loop.

## Sort a small numeric list

```indium run
fn sorted_copy(values: list) -> list {
    result = values + []
    count = len(result)
    for pass in 0..count {
        for index in 0..(count - 1) {
            if result[index] > result[index + 1] {
                previous = result[index]
                result[index] = result[index + 1]
                result[index + 1] = previous
            }
        }
    }
    return result
}

original = [9, 2, 7, 1]
print(sorted_copy(original)) // [1, 2, 7, 9]
print(original)              // [9, 2, 7, 1]
```

This simple bubble-sort example is for small lists of mutually comparable
numbers. It is quadratic work, not a fast standard-library sort. `values + []`
makes a shallow copy so swapping outer elements does not modify the original.

## Keep state in a returned function

```indium run
fn make_counter(start: integer) -> function {
    value = start
    fn increment() -> integer {
        value += 1
        return value
    }
    return increment
}

next_score = make_counter(48)
print(next_score(), next_score()) // 49 50
```

The inner function keeps its captured `value` alive. Separate calls to
`make_counter` create independent environments. `examples/closures.in` shows
the same idea with two counters. Reference cycles are not reclaimed by a
tracing collector yet, so avoid manufacturing many cyclic long-lived graphs.

## Return a recoverable error

```indium run
fn positive(value: integer) {
    if value <= 0 { return error("must be positive") }
    return success(value)
}

for number in [-2, 49] {
    checked = positive(number)
    if checked is success {
        print("Accepted:", checked.value)
    } else {
        print("Rejected:", checked.error)
    }
}
```

Results let your caller handle a failed operation without a runtime diagnostic.
`error("message")` constructs a result; it does not throw an exception.
Conversion, bounds, and most media errors are still diagnostics, so returning
a result from your own function does not automatically catch those failures.

## Read and save a text note

This example creates a `notes` directory beside its `.in` file and appends to
`notes/log.txt` when executed. The documentation verifier only checks it.

```indium
import files
created = files.mkdir("notes")
if created is error {
    print("Could not create notes folder:", created.error)
    stop()
}

saved = files.append("notes/log.txt", "Another Indium run.\n")
if saved is error {
    print("Could not append:", saved.error)
    stop()
}

loaded = files.read("notes/log.txt")
if loaded is success {
    print(loaded.value)
} else {
    print("Could not read:", loaded.error)
}
```

Use `files.write` to replace the complete contents instead of appending. Read
and inspect the operation's result even when `files.exists` returned true;
existence is not a promise that access will succeed.

## Run a repeated task a fixed number of times

```indium run
remaining = 3
every 10 milliseconds {
    print("Remaining:", remaining)
    remaining -= 1
    if remaining == 0 { stop() }
} loop
```

Declare `remaining` outside the handler so each invocation updates persistent
state. `stop()` exits the entire program. To keep other handlers running after
one task is finished, keep a boolean flag and have that handler do nothing
when the flag is false; there is no unregister API yet.

## Validate interactive numeric input

See `examples/guessing-game.in`. Its `parse_guess` function:

1. Trims the input text.
2. Rejects empty text or more than three characters.
3. Checks every character against `"0123456789"`.
4. Converts to integer only after those checks.
5. Rejects values outside 1..100 and returns a success/error result.

This avoids trying to recover from `integer("not a number")`, which raises a
diagnostic rather than returning a result. The three-character bound also keeps
the conversion far below the 64-bit integer limit. The game expects interactive
input; interrupt it with Ctrl+C if you want to leave before the game ends.

## Work with command-line arguments

Run the supplied argument demo from the project's main folder:

```powershell
indium run examples\arguments.in -- hello "two words" 49
```

`args()` returns text values. It does not automatically parse numbers or flags.
Your own program can inspect `len(args())`, compare text flags, and validate
values before conversion. See the [CLI reference](cli.md).

## Build a small game loop

Start with the complete movement example in
[Graphics, input, and audio](graphics-and-audio.md). Put simulation in ticks,
draw in frames, and let the runtime present the frame. The shipped `2d.in`
adds bouncing at the window bounds, image drawing, audio, and fullscreen.

To add a sound on a key press, put this fragment in a tick or frame handler
after the window opens, and supply the referenced OGG file:

```indium fragment
if input.key_pressed("space") {
    audio("assets/jump.ogg", 0.5)
}
```

Use `key_pressed` for one effect per new press. `key_down` would start another
voice on every update while the key is held, quickly reaching the voice limit.
