# Windows installation

After the Windows build succeeds, double-click **install-windows.cmd** in the
main Indium folder. Alternatively, download and double-click the standalone
**Install-Indium.cmd**. The standalone installer finds the usual extracted
`indium-prealpha\indium\target\release` folder beside it; otherwise, a file
picker asks you to select your built `indium.exe`.

The installer uses the Windows executable you have already built. It does not
download or compile anything and does not require administrator permissions.

It copies `indium.exe`, `SDL2.dll`, and the license notices into:

```text
%LOCALAPPDATA%\Programs\Indium
```

The `bin` subfolder is added to your **user PATH**, with no duplicate entry on
reinstall. Other PATH entries, unexpanded environment references, and their
registry value type are preserved. Windows is notified of the new environment.

Close all terminal windows and open a new terminal. Then:

```powershell
indium --version
indium run program.in
```

Run the second command in the folder containing `program.in`, or pass a relative
or absolute path to your file. Indium itself can be found from any folder. Your
source files do not have to be placed beside `indium.exe`.

The installer configures the terminal command. It does not register a
double-click association for `.in` source files; execute those with `indium run`.

Already-running terminals and terminal host processes keep their old
environment. If a completely reopened terminal still has the old PATH, sign
out of Windows and sign back in. For immediate use in the existing PowerShell
session, you can also run:

```powershell
$env:Path += ";$env:LOCALAPPDATA\Programs\Indium\bin"
```

## Updates and removal

Run the installer again after building a new version. It updates the installed
runtime without duplicating PATH. Close programs currently using Indium before
updating or uninstalling.

To remove Indium, use **Settings > Apps > Installed apps > Indium > Uninstall**.
The uninstaller removes its installed runtime, registration, and PATH entry.
It preserves the original project build, your `.in` programs, and unknown files
inside the installation directory. It does not recursively delete the folder.

## Explicit command-line options

Run from the source project's main directory:

```powershell
.\scripts\install-windows.ps1 -SourceDir .\target\release
.\scripts\install-windows.ps1 -SourceDir .\target\release -InstallDir "$env:LOCALAPPDATA\Programs\Indium"
& "$env:LOCALAPPDATA\Programs\Indium\setup.ps1" -Uninstall
```

The installer needs both `indium.exe` and `SDL2.dll`, plus the original license
notices in the source/release folder. Building the executable still requires
Rust, the MSVC C++ build tools, and the SDL2 Visual C++ development package. See
the main README. The runtime does not need Rust once it has been built.

The scripts support Windows PowerShell 5.1. Their PATH and file operations were
tested using PowerShell Core on Linux with native Windows calls mocked. Actual
Windows registry updates, the file picker, Installed Apps integration, and
environment broadcasts still need a Windows installation smoke test.
