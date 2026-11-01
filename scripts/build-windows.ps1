param([Parameter(Mandatory=$true)][string]$Sdl2Root)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$root = (Resolve-Path -LiteralPath $Sdl2Root).Path
$library = Get-ChildItem -LiteralPath $root -Filter 'SDL2.lib' -Recurse | Where-Object { $_.Directory.Name -eq 'x64' } | Select-Object -First 1
if (-not $library) { throw 'SDL2.lib was not found in an x64 folder. Extract the SDL2 Visual C++ development package first.' }
$dll = Join-Path $library.Directory.FullName 'SDL2.dll'
if (-not (Test-Path -LiteralPath $dll)) { throw "SDL2.dll was not found beside $($library.FullName)." }
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Install the current stable x64 MSVC Rust toolchain first, then reopen PowerShell.' }
$previousEncodedFlags = $env:CARGO_ENCODED_RUSTFLAGS
try {
    # RUSTFLAGS is split on whitespace; quotes inside it are literal characters.
    # Encoded flags preserve a complete library path, including spaces, as one argument.
    $flags = @()
    if ($previousEncodedFlags) {
        $flags = @($previousEncodedFlags.Split([char]31))
    } elseif ($env:RUSTFLAGS) {
        $flags = @($env:RUSTFLAGS -split '\s+' | Where-Object { $_ })
    }
    $flags += '-Lnative=' + $library.Directory.FullName
    $env:CARGO_ENCODED_RUSTFLAGS = $flags -join [char]31
    Push-Location $projectRoot
    try {
        & cargo build --release --locked
        if ($LASTEXITCODE -ne 0) { throw 'Cargo build failed.' }
        Copy-Item -LiteralPath $dll -Destination (Join-Path $projectRoot 'target\release\SDL2.dll') -Force
        Write-Host 'Built target\release\indium.exe. Run: .\target\release\indium.exe run examples\hello.in'
    } finally { Pop-Location }
} finally { $env:CARGO_ENCODED_RUSTFLAGS = $previousEncodedFlags }
