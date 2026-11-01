"""Check handbook links/API coverage and execute explicitly marked examples.

Fence markers: indium (check), indium run, indium frames=N, indium error,
indium fragment. File/network/interactive snippets are checked unless a separate
fixture invocation below supplies controlled inputs. No public HTTP requests.
"""

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile


FENCE = re.compile(r"^```indium(?P<info>[^\n]*)\n(?P<code>.*?)^```[ \t]*$", re.M | re.S)
LINK = re.compile(r"(?<!!)\[[^\]]*\]\(([^)]+)\)")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--interpreter", type=Path)
    parser.add_argument("--graphics", action="store_true", help="Run frames=N examples with offscreen Mesa")
    parser.add_argument("--report", type=Path)
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    executable = args.interpreter
    if executable is None:
        executable = root / ("target/release/indium.exe" if os.name == "nt" else "indium-linux")
    executable = executable.resolve()
    if not executable.is_file():
        raise SystemExit(f"Interpreter not found: {executable}; pass --interpreter PATH")
    documents = [root / "README.md", *sorted((root / "docs").glob("*.md"))]
    text = "\n".join(path.read_text(encoding="utf-8") for path in documents)
    api_source = (root / "src/stdlib/mod.rs").read_text(encoding="utf-8")
    functions_region = api_source.split("fn known_function", 1)[1].split("pub fn set_member", 1)[0]
    functions = set(re.findall(r'"([a-z_]+\.[a-z_]+)"', functions_region))
    initial_region = api_source.split('for n in ["window"', 1)[0]
    functions.update(re.findall(r'"([a-z_]+)"', initial_region))
    functions.update({"integer", "decimal", "text", "boolean", "list", "map", "function", "any", "success", "error", "input", "audio"})
    property_region = api_source.split("pub fn member", 1)[1].split("fn known_function", 1)[0]
    properties = {f"{namespace}.{name}" for namespace, name in re.findall(r'\("([a-z_]+)", "([a-z_]+)"\)', property_region)}
    missing = sorted(name for name in functions | properties if name not in text)
    if missing:
        raise SystemExit("Undocumented public APIs: " + ", ".join(missing))

    links = 0
    for path in documents:
        prose = re.sub(r"^```[^\n]*\n.*?^```[ \t]*$", "", path.read_text(encoding="utf-8"), flags=re.M | re.S)
        for target in LINK.findall(prose):
            if re.match(r"^[a-zA-Z]+:", target) or target.startswith("#"):
                continue
            target = target.split("#", 1)[0].strip("<>")
            if target and not (path.parent / target).is_file():
                raise SystemExit(f"Broken local link in {path.relative_to(root)}: {target}")
            links += 1

    records = []
    checked = executed = expected_errors = fragments = graphical = 0
    with tempfile.TemporaryDirectory(prefix="indium-docs-") as temporary:
        work = Path(temporary)
        environment = os.environ.copy()
        environment.update(SDL_VIDEODRIVER="offscreen", LIBGL_ALWAYS_SOFTWARE="1", SDL_AUDIODRIVER="dummy")

        def command(arguments, source, input_text=""):
            return subprocess.run([str(executable), *arguments], cwd=work, env=environment,
                                  input=input_text, capture_output=True, text=True, encoding="utf-8", timeout=10)

        for document in documents:
            content = document.read_text(encoding="utf-8")
            for match in FENCE.finditer(content):
                mode = match["info"].strip() or "check"
                line = content[:match.start()].count("\n") + 1
                location = f"{document.relative_to(root)}:{line}"
                if mode == "fragment":
                    fragments += 1
                    continue
                if mode not in {"check", "run", "error"} and not re.fullmatch(r"frames=\d+", mode):
                    raise SystemExit(f"Unknown documentation fence mode {mode!r} at {location}")
                source = work / f"snippet-{checked}.in"
                source.write_text(match["code"], encoding="utf-8")
                result = command(["check", str(source)], source)
                checked += 1
                if mode == "error":
                    if result.returncode == 0:
                        result = command(["run", str(source), "--max-steps", "100000"], source)
                    if result.returncode == 0:
                        raise SystemExit(f"Expected an error at {location}, but the example succeeded")
                    expected_errors += 1
                else:
                    if result.returncode:
                        raise SystemExit(f"Check failed at {location}:\n{result.stderr}")
                    should_run = mode == "run" or (mode.startswith("frames=") and args.graphics)
                    if should_run:
                        options = ["run", str(source), "--max-steps", "100000"]
                        if mode.startswith("frames="):
                            options += ["--frames", mode.split("=", 1)[1]]
                            graphical += 1
                        result = command(options, source)
                        if result.returncode:
                            raise SystemExit(f"Run failed at {location}:\n{result.stderr}")
                        executed += 1
                records.append({"location": location, "mode": mode, "stdout": result.stdout, "stderr": result.stderr})
                if args.verbose:
                    print(f"PASS {location} ({mode}): {result.stdout.strip()!r}")

        # Keep example assets/modules together in a disposable copy.
        example_copy = work / "examples"
        shutil.copytree(root / "examples", example_copy)
        examples = sorted(example_copy.rglob("*.in"))
        for example in examples:
            result = command(["check", str(example)], example)
            if result.returncode:
                raise SystemExit(f"Shipped example failed checking: {example.name}\n{result.stderr}")
        cases = [
            ("arguments.in", ["--", "hello", "two words"], "", "Number of arguments: 2\nhello\ntwo words\n"),
            ("closures.in", [], "", "49 50 100\n"),
            ("results.in", [], "", "Answer: 4\nHandled error: Cannot divide by zero\n"),
            ("guessing-game.in", [], "1\n" * 7, None),
        ]
        for filename, extra, supplied_input, expected in cases:
            result = command(["run", str(example_copy / filename), "--max-steps", "100000", *extra], example_copy / filename, supplied_input)
            if result.returncode or (expected is not None and result.stdout != expected):
                raise SystemExit(f"Tutorial example failed: {filename}\n{result.stderr}\n{result.stdout}")

    summary = {"documents": len(documents), "documented_functions": len(functions),
               "documented_properties": len(properties), "local_links": links,
               "checked_snippets": checked, "executed_snippets": executed,
               "expected_error_snippets": expected_errors, "fragment_snippets": fragments,
               "offscreen_graphics_snippets": graphical, "checked_example_files": len(examples),
               "executed_tutorial_files": len(cases)}
    if args.report:
        args.report.write_text(json.dumps({"summary": summary, "snippets": records}, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
