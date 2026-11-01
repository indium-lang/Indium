# Linux executable

`linux-x86_64/indium` is the optimized, tested Linux x86-64 Indium interpreter.
It is not a Windows `.exe` and is not the native compilation of an Indium source
program. It was built on Ubuntu 24.04 with glibc 2.39 and links to SDL2. A copy of
SDL2 2.30 is included in `linux-x86_64/lib`; its notices are in the project's
THIRD_PARTY-LICENSES.txt. Other normal desktop Linux runtime libraries are still
system dependencies.

From the project's extracted root, use the launcher to select the included SDL2:

```sh
chmod +x indium-linux prebuilt/linux-x86_64/indium
./indium-linux --version
./indium-linux run examples/hello.in
./indium-linux run examples/2d.in
```

An x86-64 Linux environment with a working OpenGL display is needed for windows.
Headless verification used Mesa's offscreen OpenGL driver. Audio uses the actual
system device unless SDL_AUDIODRIVER=dummy is explicitly selected for testing.
If an older distribution cannot load this binary, build from source there.
Windows/macOS users should build from source following the main README; binaries
for those platforms are not included in this snapshot. A Windows build and the
terminal hello example have been confirmed working by the user. After building
on Windows, double-click `install-windows.cmd` to install it on your user PATH.
