"""Package tracked source, existing Linux binaries, and a standalone Windows setup.

This does not build or claim to provide a Windows executable. The Windows
installer uses a previously built indium.exe and its adjacent SDL2.dll.
"""

import argparse
import hashlib
from pathlib import Path
import subprocess
import zipfile


def standalone_installer(project: Path) -> bytes:
    payload = (project / "scripts/install-windows.ps1").read_text(encoding="utf-8")
    loader = (
        "$source = [IO.File]::ReadAllText($env:INDIUM_SETUP_SELF); "
        "$payload = ($source -split '(?m)^#==INDIUM-POWERSHELL==\\r?$', 2)[1]; "
        "& ([scriptblock]::Create($payload)) "
        "-PackageRoot ([IO.Path]::GetDirectoryName($env:INDIUM_SETUP_SELF)) "
        "-InstallerText $payload"
    )
    header = (
        "@echo off\n"
        "setlocal\n"
        'set "INDIUM_SETUP_SELF=%~f0"\n'
        'powershell.exe -NoLogo -NoProfile -STA -ExecutionPolicy Bypass -Command "'
        + loader
        + '"\n'
        'set "indiumSetupExit=%ERRORLEVEL%"\n'
        "echo(\n"
        "pause\n"
        "exit /b %indiumSetupExit%\n"
        "#==INDIUM-POWERSHELL==\n"
    )
    return (header + payload).replace("\r\n", "\n").replace("\n", "\r\n").encode("utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    project = Path(__file__).resolve().parent.parent
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    installer = output / "Install-Indium.cmd"
    installer.write_bytes(standalone_installer(project))

    tracked = subprocess.check_output(
        ["git", "ls-files", "-z"], cwd=project
    ).decode().split("\0")
    files = {name: (project / name).read_bytes() for name in tracked if name}
    for name in (
        "prebuilt/linux-x86_64/indium",
        "prebuilt/linux-x86_64/lib/libSDL2-2.0.so.0",
    ):
        files[name] = (project / name).read_bytes()
    guide = project / "docs/guide.html"
    if not guide.is_file():
        raise RuntimeError("Build the offline handbook first: node scripts/build-guide.mjs")
    files["docs/guide.html"] = guide.read_bytes()
    commit = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=project, text=True
    ).strip()
    bundle = output / "indium-local-history.bundle"
    subprocess.run(
        ["git", "bundle", "create", str(bundle), "--all"], cwd=project, check=True
    )
    files["local-history.bundle"] = bundle.read_bytes()
    files["BUILD_INFO.txt"] = (
        "Indium 0.1.0-alpha.1\n"
        f"Source commit: {commit}\n"
        "Source changes and history are local. No GitHub pushes.\n"
        "Linux runtime: Ubuntu 24.04 x86-64, Rust 1.99.1, SDL2 2.30.\n"
        "Windows setup installs an existing Windows build and adds user PATH.\n"
        "Windows executable is not included; the user has built and run it locally.\n"
        "Installer logic: tested on PowerShell Core/Linux with native Windows calls mocked.\n"
        "Native Windows registry, GUI, and environment broadcast: not executed here.\n"
    ).encode()
    for name in list(files):
        if name.lower().endswith(".cmd"):
            files[name] = files[name].replace(b"\r\n", b"\n").replace(b"\n", b"\r\n")
    files["SHA256SUMS.txt"] = "".join(
        f"{hashlib.sha256(data).hexdigest()}  {name}\n"
        for name, data in sorted(files.items())
    ).encode()
    archive = output / "indium-prealpha.zip"
    with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as zf:
        for name, data in sorted(files.items()):
            info = zipfile.ZipInfo("indium/" + name, date_time=(2026, 10, 29, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = (0o100755 if name in {"indium-linux", "prebuilt/linux-x86_64/indium"} else 0o100644) << 16
            zf.writestr(info, data)
    with zipfile.ZipFile(archive) as zf:
        bad = zf.testzip()
        if bad:
            raise RuntimeError(f"Archive CRC check failed: {bad}")
        manifest = zf.read("indium/SHA256SUMS.txt").decode()
        for line in manifest.splitlines():
            expected, name = line.split("  ", 1)
            actual = hashlib.sha256(zf.read("indium/" + name)).hexdigest()
            if actual != expected:
                raise RuntimeError(f"Archive hash mismatch: {name}")
    print(f"Created {archive}: {archive.stat().st_size} bytes, {len(files)} files; CRC and SHA256 checks passed")
    print(f"Created {installer}: {installer.stat().st_size} bytes")


if __name__ == "__main__":
    main()
