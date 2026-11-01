# Language reference

This chapter describes the implemented language. For a first program, read
[Getting started](getting-started.md). For function signatures, use the
[Standard library](standard-library.md).

Indium source files use UTF-8 and the `.in` extension. Newlines separate
statements. Semicolons are optional separators when placing statements on one
line. Blocks use braces. Comments use `//`, `#`, or nested `/* ... */`.
Save UTF-8 without a BOM; the lexer does not currently strip one. UTF-16 source
files are not accepted.

## Modes

```indium
mode = terminal
// Other implemented modes: background, 2d
```

Terminal is the default. `2d` creates an OpenGL-backed window after top-level
initialization. `background` creates no graphics window, but does not detach the
process or suppress an inherited console. `3d` is recognized and explicitly
rejected as unimplemented. Set the mode before the scheduler starts; changing
execution mode while a program is running is unsupported.

## Variables and values

```indium
count = 49
speed = 2.5
name = "Indium"
ready = true
missing = none
score: integer = 0
position: decimal = 0
anything: any = 49
anything = "can change type"
```

A variable's first assignment defines it in the current lexical scope.
Reassignment updates the nearest existing binding. Otherwise, the assignment
creates a new local binding. Each block and function introduces a scope.

| Annotation | Values it accepts | Typical value |
| --- | --- | --- |
| `integer` | Signed 64-bit whole numbers | `49` |
| `decimal` | Finite floating-point numbers; integer assignments widen | `2.5` |
| `text` | Immutable Unicode text | `"hello"` |
| `boolean` | Exactly true or false | `true` |
| `list` | Lists and this implementation's lazy ranges | `[1, 2]` |
| `map` | Maps; local module values also use this coarse runtime category | `{name: "Indium"}` |
| `function` | User functions and built-in function values | A function name |
| `none` | Only the absence value | `none` |
| `any` | Any value | Use for intentionally changing types |

These are the complete annotation names. There are no generics such as
`list<integer>`, nullable-type syntax, or separate `result`/`module` annotations.
Lists/maps do not enforce an element/field schema. Several special runtime
values use the coarse `any` category internally; `type_of` provides extra names
such as `"result"` and `"module"`. A built-in namespace and a local module are
different internal value kinds, so do not rely on them sharing a `type_of` name.

Inferred variable types remain fixed. A `decimal` binding accepts an integer by
widening it; an `integer` binding does not silently accept decimals or text.
An `any` annotation explicitly allows varying types. Function parameters can be
annotated or inferred from each call. An unannotated function's return values
may have different types across calls.

For a value that starts absent and is filled in later, declare it `any`:

```indium run
selected: any = none
selected = "first item"
print(selected)
```

Without the annotation, a variable initialized with `none` can only hold `none`.
`any(value)` is an identity conversion; it does not change a variable's declared
type. A typed parameter accepts the same assignment rules as a typed variable.
An unannotated parameter gets its runtime binding type from that call's value;
use `parameter: any` if its type must change within the function.

Integers are signed 64-bit. Overflow raises an error. Decimals are finite 64-bit
floating-point values. Division always produces a decimal; division by zero,
non-finite results, and invalid mathematical domains raise errors. Numeric
comparison between an integer and a decimal can lose integer precision; keep
large integers as integers. Integer/integer equality and ordering remain exact;
integer/decimal equality checks avoid rounding a large integer into a false
match.

Numeric literals may contain digit-separating underscores (`1_000_000`) and
decimal exponents (`1.25e-3`). Use `0.5`, not `.5`, and `2.0`, not `2.`. Hex/binary
numeric literals and `NaN`/infinity literals are not implemented. An integer
literal must fit before unary minus is applied: the minimum signed integer can
be obtained with `integer("-9223372036854775808")`, while spelling that magnitude
directly as a negative source literal currently fails in the lexer.

An integer result stays integer for integer `+`, `-`, `*`, `%`, and valid integer
`**`. Mixing a decimal into numeric arithmetic produces a decimal. `/` always
produces a decimal, even for two integers. Decimal arithmetic has ordinary
floating-point rounding; `0.1 + 0.2` should not be used as an exact decimal-money
representation. There is no arbitrary-precision or fixed-point numeric type.

Text accepts single or double quotes. Escapes: `\n`, `\r`, `\t`, `\0`, `\\`,
`\"`, `\'`. Text literals cannot span physical lines. Indexing and length count
Unicode scalar values, not bytes or grapheme clusters.

For example, `len("äλ")` is 2 and `"äλ"[-1]` is `"λ"`. A visible character made
from multiple combining scalars can have a length greater than 1. There is no
raw-string prefix, string interpolation, triple-quoted literal, or `\u` escape;
write Unicode characters directly in the UTF-8 source. Join strings with `+`
and `text(value)`, or print separate values with `print`.

## Operators

From tighter to looser binding:

1. Calls, member access, indexing.
2. `**` exponentiation (right associative).
3. Unary `+`, unary `-`.
4. `*`, `/`, `%`.
5. `+`, `-`.
6. `..` (end-exclusive integer range).
7. `<`, `<=`, `>`, `>=`, `is`.
8. `==`, `!=`.
9. `not` / `!`.
10. `and` / `&&`.
11. `or` / `||`.

`not` and `!` negate the following comparison, so `not x is integer` means
`not (x is integer)`. Conditions and logical operators require booleans;
Indium does not implicitly treat numbers, lists, or text as true/false.
Logical operators short-circuit. Use `a < b and b < c` rather than chained
comparisons. Compound assignments: `+=`, `-=`, `*=`, `/=`, `%=`.

Text concatenates with text using `+`. Lists concatenate with lists. Use
`text(value)` when explicitly building text; `print` accepts values directly.
Ranges iterate upwards and exclude the end. A range whose start exceeds its
end is empty.

```indium run
print(2 + 3 * 4)        // 14
print(2 ** 3 ** 2)      // 512: 2 ** (3 ** 2)
print(-2 ** 2)          // -4: -(2 ** 2)
print(5 / 2)            // 2.5
print(7 % 3)            // 1
print(decimal(2) ** -1) // 0.5
print(not 49 is text)   // true
```

For integer `base ** exponent`, the exponent must be an integer from 0 through
4294967295, and the result must fit in 64 bits. Use a decimal base for negative
or fractional powers. `math.pow` is another decimal-power option after importing
math. There is no `**=` assignment, integer-division operator, bitwise operator,
increment/decrement operator, ternary operator, or expression-form `if`.

Compound assignment requires an already-existing target. `count += 1` does not
declare `count`. An indexed/member assignment target is evaluated once before
its right-hand side, so side effects in an index are not repeated.

`==` and `!=` can compare unlike types and return false/true as appropriate;
numeric integer/decimal equality is the special numeric case described above.
Ordering supports numbers and text with text. Text ordering is lexical, not
locale-aware alphabetical sorting. Functions compare by identity, not by body.

Logical operators require booleans on both evaluated sides. They do not return
an arbitrary operand as a shortcut value. `false and dangerous()` and
`true or dangerous()` do not call `dangerous`.

## Type and state checks

```indium
print(49 is integer)
print("hello" is text)
print(none is none)

answer = success(49)
if answer is success {
    print(answer.value)
}
problem = error("something failed")
if problem is error {
    print(problem.error)
}
```

`is` checks a type, `none`, or a result variant. `==` compares values. They are
not interchangeable. Applying `is success` to a non-result value is an error.
Only the matching `.value` or `.error` result field can be read; `.ok` is always
a boolean. Result constructors and functions returning results are ordinary
values, not a general exception-handling system.

There is no automatic result unwrapping, `try`/`catch`, `throw`, or `?` operator.
User functions can return results too:

```indium run
fn divide(a, b) {
    if b == 0 {
        return error("cannot divide by zero")
    }
    return success(a / b)
}

answer = divide(12, 3)
if answer.ok {
    print(answer.value) // 4
} else {
    print(answer.error)
}
```

`success(value)` and `error(value)` each take exactly one value. The payload can
be any value, not just text. Use `is success`/`is error` or `.ok` before reading
the matching field. `value is any` is always true; `none is none` tests absence.
`is` is not an object-identity operator, and `value is "hello"` is invalid.

## Collections

```indium
items = [1, 2, "three"]
point = (10, 20) // parentheses with commas also form a list
player = {name: "flight", score: 0}
items[0] = 49
player.score += 10
player["active"] = true
push(items, 240)
print(items[-1], len(items), player.name)
```

Lists and maps are mutable reference values; assigning a collection to another
variable aliases it. Elements can contain different types. Map keys are text;
`record.field` is shorthand for `record["field"]`. Map iteration visits keys in
sorted order. Invalid indexes and missing keys raise errors. A negative index
counts from the end. Text is immutable. `items.length` and `name.length` also
report length.

Lists/maps compare structurally, with a bounded depth for cyclic structures.
Printing deeply nested/cyclic structures truncates after eight levels.

List and text indexes start at zero. `-1` is the last item, `-2` the previous
item. Reading or assigning outside the existing list bounds fails; assignment
does not append. Use `push` to append and `pop` to remove the last item. There
are no slicing, list-comprehension, `.push()`, or direct text-mutation methods.

`{}` and `[]` create empty collections. Map literal keys may be bare names or
quoted text, but cannot be arbitrary key expressions. Use indexed assignment
for a computed key. Duplicate keys in a map literal are rejected. Updating an
existing key replaces its value; assigning a new text key adds it. A map does
not preserve insertion order and has no key-deletion API yet.

```indium run
items = [10, 20]
alias = items
alias[0] = 49
copy = items + []
copy[0] = 99
print(items[0], copy[0]) // 49 99

player = {}
field = "score"
player[field] = 240
print(player.score)    // 240
print(contains(player, "missing")) // false
```

List concatenation creates a new outer list, but nested lists/maps remain
shared references: it is a shallow copy, not a deep clone. `contains` tests a
map key's presence before you access a possibly missing field. Map iteration
and `keys(map)` visit sorted text keys.

`start..end` creates a lazy ascending range of integers. It can be iterated and
passed to `len`, but is not an indexable/materialized list. `range is list` and
`type_of(range)` currently use the coarse list category; `list(range)` does not
expand it into items. To materialize it, push each value into a new list.

## Functions and control flow

```indium
ready = true
count = 3

fn add(a, b) {
    return a + b
}

fn double(n: integer) -> integer {
    return n * 2
}

if ready {
    print(double(49))
} else if count > 10 {
    print("more than ten")
} else {
    print("waiting")
}

for n in 0..10 {
    if n == 3 { continue }
    if n == 8 { break }
    print(n)
}

while count > 0 {
    count -= 1
}

loop {
    break
}
```

Functions are visible throughout their enclosing block, so mutual recursion
works. Variable initialization still occurs in source order. Calling a function
before a captured variable has been initialized produces a runtime diagnostic.
Functions are first-class values and retain lexical captures. A function without
a return statement returns `none`. `break` and `continue` apply only to the
current function's loops.

For-loops snapshot a list/map's iteration values at loop entry. Mutating the
original collection does not change the iteration sequence. Ranges are lazy.

`for` accepts ranges, lists, text, or maps. Text iteration gives one-scalar text
values; map iteration gives keys, so fetch a value with `map[key]`. There is no
two-variable key/value loop syntax. Each loop iteration has a fresh block scope.

An assignment to an existing outer name updates that binding, even inside an
`if` or function. A type annotation does not force a fresh shadowing binding.
Function parameters and `for` iteration variables create their own bindings.

```indium run
total = 0
if true {
    total += 5
    temporary = "only inside this block"
}
print(total) // 5
// Accessing temporary here would fail: it is out of scope.
```

There is no `let`/`var`/`const` declaration syntax. Built-in bindings, imported
aliases, and declared function names are read-only; normal variables are mutable.
Use a different name rather than trying to overwrite a built-in such as `print`.

## Functions as values and closures

Pass a function by writing its name without parentheses. Add parentheses to
call it. User functions require exactly the declared number of positional
arguments. They have no optional, variadic, default, or named parameters.

The `function` annotation accepts actual user/built-in function values. Some
other kinds of value are also callable, including conversion types, `input`,
`audio`, and result constructors, but are not in that runtime type category.
Use `any` for a parameter meant to accept those general callable values.

```indium run
fn twice(value: integer) -> integer {
    return value * 2
}

fn apply(operation: function, value) {
    return operation(value)
}

chosen = twice
print(apply(chosen, 21)) // 42
```

A function can declare and return another named function. It retains access
to the environment where it was created, even after the outer function returns:

```indium run
fn make_counter() {
    value = 0
    fn next() {
        value += 1
        return value
    }
    return next
}

counter = make_counter()
another = make_counter()
print(counter(), counter(), another()) // 1 2 1
```

These are lexical closures. There are no anonymous/lambda function expressions
yet. A function name is available throughout its enclosing block, which allows
forward calls and mutual recursion. Captured variables still need to have been
initialized before the call uses them. Recursive algorithms must respect the
runtime's depth limits.

`return` ends the function immediately, including from nested loops. Bare
`return` and falling off the end return `none`; an annotated non-`none` return
type will reject that value. There is no implicit return of the final expression.
`return` at top level or directly inside a recurring handler is invalid.

## Imports

```indium fragment
import math
import files as fs
import game
import "modules/geometry.in" as geometry
```

Built-in modules: `math`, `files`, `net`, `time`, `game`, `text`.
Window, rendering, input, and audio APIs are available without imports.
`text` also remains a conversion/type name; its text helpers are accessible as
members, with or without `import text`.

Local module paths are relative to the importing source file. Their extension
must be `.in`. Without an alias, the filename stem is used. Imports are checked
recursively, loaded once, and cached by canonical path. Circular imports are
reported. Import errors preserve the failing file and source position.

A module exports its own declarations except names starting with `_`. Built-in
names inherited by a module are not re-exported. Module fields can be read;
mutating module state should happen through its exported functions. A module
function's file and asset paths remain relative to the defining module.

Use `as` to choose a convenient alias or fix a filename that is not a valid
identifier. Without an alias, a local file's stem is used, with hyphens changed
to underscores. Local modules are not found by a package search path; use an
explicit relative or absolute `.in` path.

All local imports are loaded and checked before execution, including imports
inside functions or a branch that would not run. A module's top-level statements
execute when its import actually runs, once per canonical file path. Reimporting
the same file under another alias shares its state rather than creating a copy.

Local module fields cannot be directly rebound with `module.score = 10`.
Exported functions can update their captured module variables. Exported
collections are reference values, so mutating an exported list/map affects the
shared collection. Names starting with `_` are inaccessible through exports,
not a general security boundary around the file's behavior.

See `examples/imports.in` and `examples/modules/geometry.in` for a runnable
two-file program. Use simple filenames and keep modules in a project folder
alongside their assets.

## Recurring execution

```indium
mode = 2d
import game
import time
game.tps = 240
fn update() {}

every frame {
    renderer.clear("black")
} loop

every tick {
    update()
} loop

every 5 seconds {
    print("Ping")
} loop
```

Handlers must be declared at module level after the variables they use. The
trailing `loop` token is required. Timers also accept `second`, `milliseconds`,
and `millisecond`. An interval expression is evaluated once at registration.

All top-level initialization finishes before the scheduler begins. Tick, frame,
and timer handlers share the language's normal lexical scopes. Statements inside
a handler run in a fresh block scope each invocation; assignments to existing
outer variables persist.

Simulation uses an accumulator and `game.dt = 1 / game.tps`. Render timing uses
`time.dt`. Rendering FPS does not set simulation TPS. The scheduler sleeps until
the next relevant deadline instead of waiting for a render frame before
considering ticks. A blocking render, HTTP call, sleep, or long handler can still
delay work because the runtime is single-threaded.

`game.max_catchup` defaults to 8 ticks per scheduler iteration. Missed tick time
beyond that bound is discarded and counted in `game.dropped_ticks`; it is not
replayed in an unbounded catch-up loop. Raise the bound for high TPS with very
low FPS. An overdue timer fires once and advances to its next scheduled interval,
skipping missed invocations. This keeps its cadence without bursts.

Call `stop()`, `game.stop()`, or `window.close()` to finish. Closing the native
window also exits. A terminal/background program with handlers remains alive
until stopped; one without handlers finishes after its statements run.

See [Ticks, frames, and timers](scheduling.md) for the lifecycle, exact counters,
overload behavior, timer interval limits, and a complete stopped timer example.

## Names, layout, and syntax boundaries

Identifiers are case-sensitive. They start with `_` or a Unicode alphabetic
character, followed by alphabetic/numeric characters or `_`. `score` and `Score`
are different names. Avoid reserved words and built-in names for your variables.

Control words are `fn`, `if`, `else`, `while`, `for`, `in`, `loop`, `break`,
`continue`, `return`, `import`, `as`, `every`, and `is`. `true`, `false`, `none`,
mode names, and `success`/`error` also have special language meaning.
`and`/`or`/`not` are operators. Tick/frame and time-unit words are interpreted
in their handler context.

Lists, maps, calls, and parameter lists support comma-separated multiline
layout and trailing commas. Parentheses containing a comma form a list:
`(49,)` is a one-item list, `(49)` groups a value, and `()` is not an empty-list
literal. Use `[]` for an empty list.

Keep binary operators with the expression they continue, for example end a
continued line with `+`. A newline that is not consumed by a larger expression
separates statements. There is no backslash line-continuation syntax.

Some familiar-looking forms are intentionally absent: assignment is a
statement rather than an expression; `=` does not test equality; `in` belongs
to a `for` loop rather than a membership expression; and there is no `match`,
`switch`, `do/while`, `elif`, `class`, or `struct` statement.

## Implementation limits

- Interpreted execution; no native compilation yet.
- Syntax nesting/chains are limited to 128, local import depth to 64.
- At most 32 nested execution blocks/function calls and 64 simultaneous expression evaluations;
  either guard may trigger first. There is no tail-call optimization.
- `--max-steps N` optionally bounds execution; normal runs have no instruction
  budget. `--frames N` bounds rendered frames for graphics smoke tests.
- Names and known scalar types are checked before execution; dynamic module,
  collection, and callable values receive further checks at runtime.
- Closures and cyclic collections use reference counting, without a tracing
  collector. Reference cycles can retain memory for the life of the process.
- No user-defined structs/classes/enums, named/default arguments, destructuring,
  exception syntax, async language features, or package installation yet.
