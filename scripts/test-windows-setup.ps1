#Requires -Version 5.1
# Runs on PowerShell Core/Linux too. Native registry, notification, and runtime
# calls are explicitly mocked; file copies and PATH transforms are real.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0
$project = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'install-windows.ps1')
$script:checks = 0

function Assert-Setup([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw "FAILED: $Message" }
    $script:checks++
    Write-Host "PASS: $Message"
}

function Assert-SetupThrows([scriptblock]$Action, [string]$Message) {
    $threw = $false
    try { & $Action } catch { $threw = $true }
    Assert-Setup $threw $Message
}

$fixture = Join-Path ([IO.Path]::GetTempPath()) ('indium-setup-test-' + [Guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($fixture)
$originalEncoded = $env:CARGO_ENCODED_RUSTFLAGS
$originalFlags = $env:RUSTFLAGS
$originalTestHome = $env:INDIUM_SETUP_TEST_HOME
try {
    $bin = 'C:\Users\Test User\AppData\Local\Programs\Indium\bin'
    Assert-Setup ((Add-IndiumPathEntry '' $bin) -ceq $bin) 'first installation creates a PATH entry'
    $raw = '%USERPROFILE%\tools;C:\Other Tools;;'
    $added = Add-IndiumPathEntry $raw $bin
    Assert-Setup ($added -ceq ($raw + ';' + $bin)) 'PATH preserves literal variables, spaces, and empty entries'
    Assert-Setup ((Add-IndiumPathEntry $added $bin) -ceq $added) 'repeat installation does not duplicate PATH'
    $equivalent = '"c:/users/test user/appdata/local/programs/indium/bin/"'
    Assert-Setup ((Add-IndiumPathEntry $equivalent $bin) -ceq $equivalent) 'PATH comparison tolerates case, quotes, and separators'
    $env:INDIUM_SETUP_TEST_HOME = 'C:\Users\Test User'
    $variablePath = '%INDIUM_SETUP_TEST_HOME%\AppData\Local\Programs\Indium\bin'
    Assert-Setup ((Add-IndiumPathEntry $variablePath $bin) -ceq $variablePath) 'PATH comparison expands existing environment references'
    $longPath = (1..1500 | ForEach-Object { 'C:\Other\Tool' + $_ }) -join ';'
    Assert-Setup ((Remove-IndiumPathEntry (Add-IndiumPathEntry $longPath $bin) $bin) -ceq $longPath) 'long PATH values survive adding and removing Indium intact'
    Assert-Setup ((Remove-IndiumPathEntry ($bin + ';C:\Other;' + $equivalent + ';') $bin) -ceq 'C:\Other;') 'uninstall removes only equivalent Indium entries'

    $source = Join-Path $fixture 'source with spaces'
    $release = Join-Path $source 'target/release'
    [void][IO.Directory]::CreateDirectory($release)
    [void][IO.Directory]::CreateDirectory((Join-Path $source 'assets'))
    foreach ($name in @('LICENSE', 'THIRD_PARTY.md', 'THIRD_PARTY-LICENSES.txt', 'assets/DejaVu-LICENSE.txt')) {
        Set-Content -LiteralPath (Join-Path $source $name) -Value ('fixture notice ' + $name)
    }
    Set-Content -LiteralPath (Join-Path $release 'indium.exe') -Value '0.1.0-alpha.1'
    Set-Content -LiteralPath (Join-Path $release 'SDL2.dll') -Value 'fixture SDL'
    $destination = Join-Path $fixture 'install with spaces'
    $destinationBin = Join-Path $destination 'bin'
    $script:fakePath = $raw
    $script:fakeRegistration = $null
    $script:notifications = 0
    function Get-IndiumRuntimeVersion([string]$Directory) {
        return (Get-Content -LiteralPath (Join-Path $Directory 'indium.exe') -Raw).Trim()
    }
    function Update-IndiumUserPath([string]$BinDir, [switch]$Remove) {
        if ($Remove) { $script:fakePath = Remove-IndiumPathEntry $script:fakePath $BinDir }
        else { $script:fakePath = Add-IndiumPathEntry $script:fakePath $BinDir }
    }
    function Register-IndiumUninstaller([string]$Directory, [string]$Version) {
        $script:fakeRegistration = @{ directory = $Directory; version = $Version }
    }
    function Unregister-IndiumUninstaller([string]$Directory) { $script:fakeRegistration = $null }
    function Send-IndiumEnvironmentChange { $script:notifications++ }
    $setupText = [IO.File]::ReadAllText((Join-Path $PSScriptRoot 'install-windows.ps1'))
    Install-Indium $release $destination $setupText
    Assert-Setup ((Get-Content -LiteralPath (Join-Path $destinationBin 'indium.exe') -Raw).Trim() -eq '0.1.0-alpha.1') 'installer copies the runtime into the permanent bin folder'
    Assert-Setup (Test-Path -LiteralPath (Join-Path $destinationBin 'SDL2.dll')) 'installer copies the SDL runtime beside Indium'
    Assert-Setup (Test-Path -LiteralPath (Join-Path $destination 'THIRD_PARTY-LICENSES.txt')) 'installer preserves third-party notices'
    Assert-Setup ($script:fakeRegistration.version -eq '0.1.0-alpha.1') 'installer registers the reported version'
    $once = $script:fakePath
    Install-Indium $release $destination $setupText
    Assert-Setup ($script:fakePath -ceq $once) 'a complete reinstall keeps PATH unchanged'
    Install-Indium $destinationBin $destination $setupText
    Assert-Setup ($script:fakePath -ceq $once) 'installing from the installed runtime avoids copying a file onto itself'
    Set-Content -LiteralPath (Join-Path $release 'indium.exe') -Value '0.1.0-alpha.2'
    Install-Indium $release $destination $setupText
    Assert-Setup (((Get-Content -LiteralPath (Join-Path $destinationBin 'indium.exe') -Raw).Trim() -eq '0.1.0-alpha.2') -and ($script:fakeRegistration.version -eq '0.1.0-alpha.2')) 'upgrades replace the runtime and update its registered version'
    Assert-Setup ($script:fakePath -ceq $once) 'upgrades do not duplicate PATH'

    $missingSource = Join-Path $fixture 'missing DLL'
    [void][IO.Directory]::CreateDirectory($missingSource)
    Set-Content -LiteralPath (Join-Path $missingSource 'indium.exe') -Value '0.1.0-alpha.1'
    $failedDestination = Join-Path $fixture 'must not be created'
    Assert-SetupThrows { Install-Indium $missingSource $failedDestination $setupText } 'missing SDL fails before installing'
    Assert-Setup (-not (Test-Path -LiteralPath $failedDestination)) 'failed source validation leaves no installation'
    Assert-Setup ($script:fakePath -ceq $once) 'failed source validation leaves PATH unchanged'

    $markerPath = Join-Path $destination 'install.json'
    $validMarker = Get-Content -LiteralPath $markerPath -Raw
    $badMarker = $validMarker | ConvertFrom-Json
    $badMarker.install_dir = $fixture
    $badMarker | ConvertTo-Json | Set-Content -LiteralPath $markerPath
    Assert-SetupThrows { Uninstall-Indium $destination } 'uninstall rejects an installation record for another folder'
    Assert-Setup (Test-Path -LiteralPath (Join-Path $destinationBin 'indium.exe')) 'rejected uninstall keeps the runtime intact'
    Set-Content -LiteralPath $markerPath -Value $validMarker

    # A link in the bin position must never redirect an uninstall outside its folder.
    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
        $savedBin = Join-Path $destination 'bin-saved'
        Move-Item -LiteralPath $destinationBin -Destination $savedBin
        [void](New-Item -ItemType SymbolicLink -Path $destinationBin -Target $release)
        Assert-SetupThrows { Uninstall-Indium $destination } 'uninstall rejects a linked bin directory'
        Assert-Setup (Test-Path -LiteralPath (Join-Path $release 'indium.exe')) 'a redirected uninstall does not delete the source runtime'
        Remove-Item -LiteralPath $destinationBin
        Move-Item -LiteralPath $savedBin -Destination $destinationBin
    }
    Set-Content -LiteralPath (Join-Path $destination 'my-program.in') -Value 'print("keep me")'
    Set-Content -LiteralPath (Join-Path $destinationBin 'user-data.txt') -Value 'keep me too'
    Uninstall-Indium $destination
    Assert-Setup (-not (Test-Path -LiteralPath (Join-Path $destinationBin 'indium.exe'))) 'uninstall removes the installed executable'
    Assert-Setup ((Test-Path -LiteralPath (Join-Path $destination 'my-program.in')) -and (Test-Path -LiteralPath (Join-Path $destinationBin 'user-data.txt'))) 'uninstall preserves unknown files and user programs'
    Assert-Setup (Test-Path -LiteralPath (Join-Path $release 'indium.exe')) 'uninstall preserves the original project build'
    Assert-Setup ($script:fakePath -ceq $raw) 'uninstall restores the unrelated PATH entries exactly'
    Assert-Setup ($null -eq $script:fakeRegistration) 'uninstall removes its application registration'
    Assert-Setup ($script:notifications -eq 5) 'install, reinstall, upgrade, and uninstall notify the environment'

    # Execute the actual build helper against a fake Cargo command. This verifies
    # argument encoding and restoration without compiling or changing this repo.
    $buildProject = Join-Path $fixture 'build fixture'
    [void][IO.Directory]::CreateDirectory((Join-Path $buildProject 'scripts'))
    [void][IO.Directory]::CreateDirectory((Join-Path $buildProject 'target/release'))
    $buildScript = Join-Path $buildProject 'scripts/build-windows.ps1'
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'build-windows.ps1') -Destination $buildScript
    $sdk = Join-Path $fixture 'SDL SDK with spaces/lib/x64'
    [void][IO.Directory]::CreateDirectory($sdk)
    Set-Content -LiteralPath (Join-Path $sdk 'SDL2.lib') -Value 'fixture link library'
    Set-Content -LiteralPath (Join-Path $sdk 'SDL2.dll') -Value 'fixture DLL'
    $global:IndiumSetupCapturedFlags = ''
    $global:IndiumSetupFakeCargoCode = 0
    function cargo {
        $global:IndiumSetupCapturedFlags = $env:CARGO_ENCODED_RUSTFLAGS
        $global:LASTEXITCODE = $global:IndiumSetupFakeCargoCode
    }
    $env:CARGO_ENCODED_RUSTFLAGS = $null
    $env:RUSTFLAGS = '-C debuginfo=1'
    $beforeDirectory = (Get-Location).Path
    & $buildScript -Sdl2Root (Split-Path -Parent (Split-Path -Parent $sdk))
    $flags = @($global:IndiumSetupCapturedFlags.Split([char]31))
    Assert-Setup (($flags.Count -eq 3) -and ($flags[2] -ceq ('-Lnative=' + $sdk))) 'build helper sends a path with spaces as one unquoted compiler argument'
    Assert-Setup (-not $env:CARGO_ENCODED_RUSTFLAGS) 'build helper restores an absent encoded flag variable'
    Assert-Setup ($env:RUSTFLAGS -ceq '-C debuginfo=1') 'build helper preserves the original RUSTFLAGS'
    Assert-Setup (Test-Path -LiteralPath (Join-Path $buildProject 'target/release/SDL2.dll')) 'successful build copies the required runtime DLL'
    $encoded = '-C' + [char]31 + 'opt-level=1'
    $env:CARGO_ENCODED_RUSTFLAGS = $encoded
    $global:IndiumSetupFakeCargoCode = 1
    Assert-SetupThrows { & $buildScript -Sdl2Root (Split-Path -Parent (Split-Path -Parent $sdk)) } 'build helper reports a failed compiler invocation'
    Assert-Setup ($env:CARGO_ENCODED_RUSTFLAGS -ceq $encoded) 'failed builds restore existing encoded flags'
    Assert-Setup ((Get-Location).Path -ceq $beforeDirectory) 'failed builds restore the caller working directory'
} finally {
    $env:CARGO_ENCODED_RUSTFLAGS = $originalEncoded
    $env:RUSTFLAGS = $originalFlags
    $env:INDIUM_SETUP_TEST_HOME = $originalTestHome
    Remove-Variable -Name IndiumSetupCapturedFlags, IndiumSetupFakeCargoCode -Scope Global -ErrorAction SilentlyContinue
    # This is the exact fresh GUID directory created by this test invocation.
    if ((Split-Path -Leaf $fixture) -match '^indium-setup-test-[a-f0-9]{32}$') {
        Remove-Item -LiteralPath $fixture -Recurse -Force
    }
}
Write-Host "$script:checks setup checks passed. Native Windows registry/GUI calls were mocked."
