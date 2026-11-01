use indium::{
    lexer::{self, Kind},
    loader, parser, run_source,
};
fn output(source: &str, expected: &str) {
    assert_eq!(
        run_source(source)
            .unwrap_or_else(|e| panic!("{}\nsource:\n{source}", e.render(Some(source)))),
        expected
    );
}
fn error(source: &str, part: &str) {
    let e = run_source(source).expect_err("program must fail");
    assert!(
        e.message.contains(part),
        "Expected '{part}', got: {}",
        e.message
    );
}
#[test]
fn hello_and_functions() {
    output(
        include_str!("../examples/hello.in"),
        "Hello, Indium!\n1 through 10 adds up to 55\n",
    );
}
#[test]
fn precedence_and_numeric_types() {
    output("print(2 + 3 * 4, 2 ** 3 ** 2, -2 ** 2, (2 + 3) * 4, 5 / 2)\nprint(1 is integer, 1.0 is decimal, none is none)","14 512 -4 20 2.5\ntrue true true\n");
}
#[test]
fn comparisons_and_short_circuit() {
    output("fn fail() { assert(false) }\nprint(false and fail(), true or fail())\nprint(3 < 4, 4 <= 4, 3 != 4, \"a\" < \"b\")","false true\ntrue true true true\n");
}
#[test]
fn inferred_types_and_widening() {
    output(
        "x = 4\nx += 2\ny: decimal = 1\ny = 2.5\nprint(x, y, y is decimal)",
        "6 2.5 true\n",
    );
    error("x = 1\nx = \"bad\"", "Expected integer");
    error("x: boolean = 2", "Expected boolean");
}
#[test]
fn conditionals() {
    output("x = 5\nif x < 2 { print(\"small\") } else if x < 10 { print(\"middle\") } else { print(\"large\") }","middle\n");
}
#[test]
fn lexical_blocks() {
    output("x = 1\nif true { x = 2\ny = 3 }\nprint(x)", "2\n");
    error(
        "if true { hidden = 2 }\nprint(hidden)",
        "Unknown variable 'hidden'",
    );
}
#[test]
fn while_break_and_continue() {
    output("n = 0\nsum = 0\nwhile n < 10 {\n n += 1\n if n == 3 { continue }\n if n == 6 { break }\n sum += n\n}\nprint(n, sum)","6 12\n");
}
#[test]
fn loops_and_ranges() {
    output(
        "n=0\nloop { n+=1\n if n==4 { break } }\nfor i in 0..4 { print(i) }\nprint(n)",
        "0\n1\n2\n3\n4\n",
    );
    output("for i in 3..0 { assert(false) }\nprint(len(3..0))", "0\n");
}
#[test]
fn recursion_and_mutual_calls() {
    output("fn even(n) { if n == 0 { return true }\nreturn odd(n - 1) }\nfn odd(n) { if n == 0 { return false }\nreturn even(n - 1) }\nfn fact(n: integer) -> integer { if n <= 1 { return 1 }\nreturn n * fact(n - 1) }\nprint(fact(10), even(10))","3628800 true\n");
}
#[test]
fn functions_are_first_class_and_capture() {
    output("fn counter() {\n x=0\n fn increment() { x+=1\n return x }\n return increment\n}\nc=counter()\nprint(c(), c(), c())","1 2 3\n");
}
#[test]
fn return_unwinds_nested_loops() {
    output(
        "fn value() { for n in 0..10 { while true { return n + 4 } } }\nprint(value())",
        "4\n",
    );
}
#[test]
fn arity_and_return_errors() {
    error("fn add(a,b){return a+b}\nadd(1)", "expects 2 arguments");
    error("fn bad() -> text { return 2 }\nbad()", "Expected text");
    error("fn bad() -> integer {}\nbad()", "Expected integer");
    error("fn typed(x:integer) {}\ntyped(\"bad\")", "Expected integer");
}
#[test]
fn lists_maps_and_unicode() {
    output("xs = [1, 2, 3]\nxs[0] += 9\npush(xs, 4)\nprint(xs[-1], pop(xs), len(xs))\nm = {name: \"Indium\", count: 49}\nm.count += 1\nm[\"extra\"] = true\nprint(m.name, m.count, m.extra)\nprint(\"kiwi ä\"[-1], len(\"äλ\"), upper(\"äλ\"))","4 4 3\nIndium 50 true\nä 2 ÄΛ\n");
}
#[test]
fn assignment_target_is_evaluated_once() {
    output("index=0\nfn next() { index+=1\nreturn index-1 }\nxs=[5,6,7]\nxs[next()] += 10\nprint(xs,index)","[15, 6, 7] 1\n");
}
#[test]
fn collection_aliases_and_cycles_do_not_panic() {
    output(
        "a=[1]\nb=a\nb[0]=5\nprint(a)\npush(a,a)\nprint(a==a)",
        "[5]\ntrue\n",
    );
}
#[test]
fn state_checks_are_distinct_from_equality() {
    output("good=success(49)\nbad=error(\"oops\")\nprint(good is success, good is error, good.value, bad.error)","true false 49 oops\n");
    error("print(\"x\" is \"x\")", "Use 'is'");
    error("print(success(1).error)", "other state");
}
#[test]
fn stdlib_math_text() {
    output("import math\nprint(math.sqrt(49), math.abs(-4), math.min(3, 8), math.clamp(30, 0, 5))\nprint(join(split(\"a,b,c\", \",\"), \" | \"), contains([1,2],2), trim(\" hi \"))","7 4 3 5\na | b | c true hi\n");
}
#[test]
fn deterministic_random() {
    output("import math\nmath.seed(49)\nx=math.random()\nmath.seed(49)\nprint(x==math.random(), math.random_int(4,4))","true 4\n");
}
#[test]
fn conversions() {
    output(
        "print(integer(\"22\"), decimal(\"2.5\"), text(49), boolean(\"false\"))",
        "22 2.5 49 false\n",
    );
    error("integer(\"not a number\")", "Cannot convert");
    error("decimal(\"NaN\")", "Cannot convert");
}
#[test]
fn out_of_bounds_and_division_errors() {
    error("print([1][5])", "out of bounds");
    error("print(4 / 0)", "Division by zero");
    error("print(4 % 0)", "zero");
    error("print(9223372036854775807 + 1)", "overflow");
    error("print(1e308 * 1e308)", "non-finite");
}
#[test]
fn integer_equality_is_exact_above_f64_precision() {
    output(
        "print(9007199254740993 == 9007199254740992, 9007199254740993 == 9007199254740992.0)",
        "false false\n",
    );
}
#[test]
fn strict_booleans_and_bad_operators() {
    error("if 1 { print(1) }", "Expected boolean");
    error("print(1 + \"x\")", "Cannot apply");
    error("print(not 1)", "Expected boolean");
}
#[test]
fn readonly_builtins_and_controls() {
    error("print=3", "read-only");
    error("break", "inside a loop");
    error("continue", "inside a loop");
    error("return 2", "inside a function");
    error("fn x() {break}\nx()", "inside a loop");
}
#[test]
fn unsupported_modes_and_modules_are_honest() {
    error("mode=3d", "not implemented");
    error("import ai", "Unknown built-in module");
    error("mode=\"2d\"", "mode must be");
}
#[test]
fn unknown_variable_location() {
    let e = run_source("x=1\nprint(nope)").unwrap_err();
    assert_eq!((e.span.line, e.span.column), (2, 7));
    assert!(e.render(Some("x=1\nprint(nope)")).contains("print(nope)"));
}
#[test]
fn tokenizer_modes_comments_and_escapes() {
    let ts = lexer::lex(
        "mode = 2d\n/* nested /* hi */ yes */\nx=1_000 + 2.5e-2\nprint(\"a\\nb\") // comment",
    )
    .unwrap();
    assert!(ts.iter().any(|t| t.kind == Kind::Ident("2d".into())));
    assert!(ts.iter().any(|t| t.kind == Kind::Integer(1000)));
    assert!(ts.iter().any(|t| t.kind == Kind::Decimal(0.025)));
    assert!(ts.iter().any(|t| t.kind == Kind::Text("a\nb".into())));
}
#[test]
fn tokenizer_errors() {
    assert!(lexer::lex("/*").is_err());
    assert!(lexer::lex("\"bad").is_err());
    assert!(lexer::lex("1e9999").is_err());
    assert!(lexer::lex("1__0").is_err());
    assert!(lexer::lex("@hi").is_err());
}
#[test]
fn parser_invalid_syntax() {
    for s in [
        "fn nope( {",
        "if true {",
        "[1,",
        "x =",
        "fn f(x,x) {}",
        "m={a:1,a:2}",
        "print(1) print(2)",
    ] {
        assert!(parser::parse(s).is_err(), "must reject {s}");
    }
}
#[test]
fn multiline_expressions_and_trailing_commas() {
    output(
        "xs=[\n 1,\n 2,\n]\nprint(\n len(xs),\n 2 +\n 3,\n)",
        "2 5\n",
    );
}
#[test]
fn recurring_loop_suffix_is_required() {
    assert!(parser::parse("every tick { print(1) }")
        .unwrap_err()
        .message
        .contains("end with 'loop'"));
    assert!(parser::parse("every tick { print(1) } loop").is_ok());
}
#[test]
fn timer_and_tick_handlers_run() {
    output("import game\ngame.tps=1000\ncount=0\nevery tick { count+=1\n if count==3 { print(game.ticks)\n stop() } } loop","3\n");
    output(
        "every 1 milliseconds { print(\"timer\")\nstop() } loop",
        "timer\n",
    );
}
#[test]
fn handlers_are_restricted_to_module_level() {
    error("fn f(){every tick {} loop}", "module level");
    error("every tick { every tick {} loop } loop", "module level");
}
#[test]
fn check_does_not_execute() {
    assert!(loader::compile_source("assert(false)\nloop {} ").is_ok());
}
#[test]
fn maximum_expression_depth_is_a_diagnostic() {
    let s = format!("print({}1{})", "(".repeat(200), ")".repeat(200));
    assert!(parser::parse(&s)
        .unwrap_err()
        .message
        .contains("nested too deeply"));
}
#[test]
fn execution_budget_stops_infinite_loop() {
    let p = loader::compile_source("loop {}").unwrap();
    let mut i = indium::interpreter::Interpreter::new(
        p,
        indium::interpreter::Options {
            capture_output: true,
            max_steps: Some(100),
            max_frames: None,
        },
    );
    assert!(i.run().unwrap_err().message.contains("step limit"));
}
#[test]
fn recursion_limit_is_a_diagnostic() {
    error("fn recurse(){return recurse()}\nrecurse()", "depth");
}
#[test]
fn text_module_and_negated_state_checks() {
    output(
        "import text\nprint(text.upper(\"hi\"), text(49), not 2 == 3, not 1 is decimal)",
        "HI 49 true true\n",
    );
}
#[test]
fn unknown_names_inside_functions_are_checked() {
    error("fn bad(){ return typo }\nbad()", "Unknown variable 'typo'");
    output("fn read(){return later}\nlater=49\nprint(read())", "49\n");
}
#[test]
fn arbitrary_long_expressions_are_bounded() {
    let source = format!("print({})", vec!["1"; 1000].join("+"));
    assert!(parser::parse(&source)
        .unwrap_err()
        .message
        .contains("nested too deeply"));
}
#[test]
fn uninitialized_function_capture_reports_a_runtime_error() {
    error(
        "fn read(){return later}\nread()\nlater=49",
        "Unknown variable 'later'",
    );
}

#[test]
fn nested_execution_is_bounded() {
    let source = format!(
        "{}print(49){}",
        "if true {\n".repeat(100),
        "\n}".repeat(100)
    );
    error(&source, "execution block depth");
}

#[test]
fn stop_during_initialization_does_not_open_a_window() {
    output("mode=2d\nstop()\nassert(false)", "");
}
