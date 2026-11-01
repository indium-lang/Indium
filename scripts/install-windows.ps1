#Requires -Version 5.1
[CmdletBinding()]
param(
    [string]$SourceDir,
    [string]$InstallDir,
    [string]$PackageRoot,
    [switch]$Uninstall,
    [Parameter(DontShow=$true)][string]$InstallerText
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0
$script:IndiumAppKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Indium'
$script:IndiumOwnedFiles = @(
    'bin/indium.exe', 'bin/SDL2.dll', 'setup.ps1', 'LICENSE',
    'THIRD_PARTY.md', 'THIRD_PARTY-LICENSES.txt', 'DejaVu-LICENSE.txt', 'install.json'
)

function Get-IndiumPathKey([string]$Entry) {
    return [Environment]::ExpandEnvironmentVariables($Entry.Trim().Trim('"')).Replace('/', '\').TrimEnd('\')
}

function Add-IndiumPathEntry([AllowEmptyString()][string]$Value, [string]$Entry) {
    $key = Get-IndiumPathKey $Entry
    foreach ($part in $Value.Split(';')) {
        if ([string]::Equals((Get-IndiumPathKey $part), $key, [StringComparison]::OrdinalIgnoreCase)) {
            return $Value
        }
    }
    if (-not $Value) { return $Entry }
    return $Value + ';' + $Entry
}

function Remove-IndiumPathEntry([AllowEmptyString()][string]$Value, [string]$Entry) {
    $key = Get-IndiumPathKey $Entry
    $remaining = @($Value.Split(';') | Where-Object {
        -not [string]::Equals((Get-IndiumPathKey $_), $key, [StringComparison]::OrdinalIgnoreCase)
    })
    return $remaining -join ';'
}

function Update-IndiumUserPath([string]$BinDir, [switch]$Remove) {
    $key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment')
    try {
        # Read unexpanded text, preserving entries such as %USERPROFILE%\bin.
        $raw = [string]$key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        $kind = [Microsoft.Win32.RegistryValueKind]::ExpandString
        if ($key.GetValueNames() -contains 'Path') { $kind = $key.GetValueKind('Path') }
        if ($kind -notin @([Microsoft.Win32.RegistryValueKind]::String, [Microsoft.Win32.RegistryValueKind]::ExpandString)) {
            throw 'Your user PATH has an unsupported registry type; it has not been changed.'
        }
        if ($Remove) { $updated = Remove-IndiumPathEntry $raw $BinDir }
        else { $updated = Add-IndiumPathEntry $raw $BinDir }
        if ($raw -cne $updated) { $key.SetValue('Path', $updated, $kind) }
    } finally { $key.Dispose() }
    if ($Remove) { $env:Path = Remove-IndiumPathEntry $env:Path $BinDir }
    else { $env:Path = Add-IndiumPathEntry $env:Path $BinDir }
}

function Send-IndiumEnvironmentChange {
    try {
        if (-not ('IndiumSetup.EnvironmentNotification' -as [type])) {
            Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
namespace IndiumSetup {
    public static class EnvironmentNotification {
        [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        public static extern IntPtr SendMessageTimeout(
            IntPtr window, uint message, UIntPtr wParam, string lParam,
            uint flags, uint timeout, out UIntPtr result);
    }
}
'@
        }
        $result = [UIntPtr]::Zero
        [void][IndiumSetup.EnvironmentNotification]::SendMessageTimeout(
            [IntPtr]0xffff, 0x001a, [UIntPtr]::Zero, 'Environment', 2, 1000, [ref]$result)
    } catch {
        Write-Warning 'PATH was saved. Sign out and back in if a new terminal cannot find Indium.'
    }
}

function Assert-IndiumInstallDirectory([string]$Directory) {
    $full = [IO.Path]::GetFullPath($Directory).TrimEnd([IO.Path]::DirectorySeparatorChar)
    $root = [IO.Path]::GetPathRoot($full).TrimEnd([IO.Path]::DirectorySeparatorChar)
    if (-not $full -or $full -eq $root) { throw 'Choose a dedicated Indium installation folder.' }
    # Refuse to install through directory junctions/symlinks.
    $parent = $full
    while ($parent) {
        if (Test-Path -LiteralPath $parent) {
            $item = Get-Item -LiteralPath $parent -Force
            if (-not $item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
                throw "Installation directory is not a normal folder: $parent"
            }
        }
        $next = Split-Path -Parent $parent
        if ($next -eq $parent) { break }
        $parent = $next
    }
    return $full
}

function Assert-IndiumInstallMarker([string]$Directory) {
    $marker = Join-Path $Directory 'install.json'
    if (-not (Test-Path -LiteralPath $marker -PathType Leaf)) {
        throw "No Indium installation record was found in $Directory."
    }
    $record = Get-Content -LiteralPath $marker -Raw | ConvertFrom-Json
    if ($record.application -ne 'Indium' -or $record.format -ne 1 -or
        -not [string]::Equals([IO.Path]::GetFullPath($record.install_dir), $Directory, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'The Indium installation record does not match this folder.'
    }
}

function Assert-IndiumOwnedFiles([string]$Directory) {
    [void](Assert-IndiumInstallDirectory (Join-Path $Directory 'bin'))
    foreach ($relative in $script:IndiumOwnedFiles) {
        $file = Join-Path $Directory $relative
        if (Test-Path -LiteralPath $file) {
            $item = Get-Item -LiteralPath $file -Force
            if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
                throw "An installed file was replaced by a directory or link: $file"
            }
        }
    }
}

function Get-IndiumRuntimeVersion([string]$Directory) {
    $output = & (Join-Path $Directory 'indium.exe') --version 2>&1
    if ($LASTEXITCODE -ne 0) { throw "The Indium executable could not start: $output" }
    $text = ($output | Out-String).Trim()
    if ($text -notmatch '^indium\s+([0-9]+\.[0-9]+\.[0-9]+[^\s]*)') {
        throw "This executable did not report an Indium version: $text"
    }
    return $Matches[1]
}

function Find-IndiumSourceDirectory([string]$Root, [string]$Requested) {
    if ($Requested) {
        $directory = (Resolve-Path -LiteralPath $Requested).ProviderPath
        if (-not (Test-Path -LiteralPath (Join-Path $directory 'indium.exe') -PathType Leaf)) {
            throw "indium.exe was not found in $directory."
        }
        return $directory
    }
    foreach ($relative in @('target/release', 'prebuilt/windows-x86_64', 'bin', '.',
            'indium/target/release', 'indium-prealpha/indium/target/release')) {
        $candidate = Join-Path $Root $relative
        if (Test-Path -LiteralPath (Join-Path $candidate 'indium.exe') -PathType Leaf) {
            return (Resolve-Path -LiteralPath $candidate).ProviderPath
        }
    }
    Add-Type -AssemblyName System.Windows.Forms
    $dialog = New-Object System.Windows.Forms.OpenFileDialog
    try {
        $dialog.Title = 'Select your built Indium executable (target\release\indium.exe)'
        $dialog.Filter = 'Indium executable (indium.exe)|indium.exe'
        $dialog.InitialDirectory = $Root
        if ($dialog.ShowDialog() -ne [System.Windows.Forms.DialogResult]::OK) {
            throw 'Installation cancelled. Select your built indium.exe to install Indium.'
        }
        return Split-Path -Parent $dialog.FileName
    } finally { $dialog.Dispose() }
}

function Find-IndiumNotices([string]$Directory) {
    $candidate = $Directory
    for ($i = 0; $i -lt 5 -and $candidate; $i++) {
        if ((Test-Path -LiteralPath (Join-Path $candidate 'LICENSE') -PathType Leaf) -and
            (Test-Path -LiteralPath (Join-Path $candidate 'THIRD_PARTY-LICENSES.txt') -PathType Leaf)) {
            return $candidate
        }
        $candidate = Split-Path -Parent $candidate
    }
    throw 'Keep the executable inside the Indium project or release folder so its license notices can be installed.'
}

function Register-IndiumUninstaller([string]$Directory, [string]$Version) {
    $powershell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
    $script = Join-Path $Directory 'setup.ps1'
    $key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($script:IndiumAppKey)
    try {
        $values = @{
            DisplayName = 'Indium'; DisplayVersion = $Version; Publisher = 'Indium'
            InstallLocation = $Directory; DisplayIcon = (Join-Path $Directory 'bin/indium.exe')
            UninstallString = ('"{0}" -NoProfile -ExecutionPolicy Bypass -File "{1}" -Uninstall -InstallDir "{2}"' -f $powershell, $script, $Directory)
        }
        foreach ($name in $values.Keys) { $key.SetValue($name, $values[$name], [Microsoft.Win32.RegistryValueKind]::String) }
        $key.SetValue('NoModify', 1, [Microsoft.Win32.RegistryValueKind]::DWord)
        $key.SetValue('NoRepair', 1, [Microsoft.Win32.RegistryValueKind]::DWord)
    } finally { $key.Dispose() }
}

function Unregister-IndiumUninstaller([string]$Directory) {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($script:IndiumAppKey)
    if (-not $key) { return }
    try { $registered = [string]$key.GetValue('InstallLocation', '') }
    finally { $key.Dispose() }
    if ([string]::Equals($registered, $Directory, [StringComparison]::OrdinalIgnoreCase)) {
        [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree($script:IndiumAppKey, $false)
    }
}

function Install-Indium([string]$Source, [string]$Directory, [string]$SetupText) {
    $Directory = Assert-IndiumInstallDirectory $Directory
    Assert-IndiumOwnedFiles $Directory
    $bin = Join-Path $Directory 'bin'
    if (Test-Path -LiteralPath (Join-Path $Directory 'install.json')) {
        Assert-IndiumInstallMarker $Directory
    } else {
        foreach ($relative in $script:IndiumOwnedFiles) {
            if (Test-Path -LiteralPath (Join-Path $Directory $relative)) {
                throw "A file already exists at $(Join-Path $Directory $relative). Choose a separate installation folder."
            }
        }
    }
    # Validate every source before changing the existing installation or PATH.
    if (-not (Test-Path -LiteralPath (Join-Path $Source 'SDL2.dll') -PathType Leaf)) {
        throw 'SDL2.dll must be beside indium.exe. Run the Windows build helper to copy it first.'
    }
    $notices = Find-IndiumNotices $Source
    $version = Get-IndiumRuntimeVersion $Source
    if (-not $SetupText) { throw 'The installer source could not be loaded.' }
    [void][IO.Directory]::CreateDirectory($bin)
    foreach ($name in @('indium.exe', 'SDL2.dll')) {
        $inputFile = [IO.Path]::GetFullPath((Join-Path $Source $name))
        $outputFile = [IO.Path]::GetFullPath((Join-Path $bin $name))
        if (-not [string]::Equals($inputFile, $outputFile, [StringComparison]::OrdinalIgnoreCase)) {
            Copy-Item -LiteralPath $inputFile -Destination $outputFile -Force
        }
    }
    foreach ($name in @('LICENSE', 'THIRD_PARTY.md', 'THIRD_PARTY-LICENSES.txt')) {
        $inputFile = Join-Path $notices $name
        $outputFile = Join-Path $Directory $name
        if ((Test-Path -LiteralPath $inputFile -PathType Leaf) -and
            -not [string]::Equals($inputFile, $outputFile, [StringComparison]::OrdinalIgnoreCase)) {
            Copy-Item -LiteralPath $inputFile -Destination $outputFile -Force
        }
    }
    $fontNotice = Join-Path $notices 'assets/DejaVu-LICENSE.txt'
    if (Test-Path -LiteralPath $fontNotice -PathType Leaf) {
        Copy-Item -LiteralPath $fontNotice -Destination (Join-Path $Directory 'DejaVu-LICENSE.txt') -Force
    }
    Set-Content -LiteralPath (Join-Path $Directory 'setup.ps1') -Value $SetupText -Encoding UTF8
    # Check the installed copy before registering the command.
    [void](Get-IndiumRuntimeVersion $bin)
    $record = [ordered]@{ application = 'Indium'; format = 1; install_dir = $Directory; version = $version }
    $record | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $Directory 'install.json') -Encoding UTF8
    Register-IndiumUninstaller $Directory $version
    Update-IndiumUserPath $bin
    Send-IndiumEnvironmentChange
    Write-Host "Installed Indium $version to $Directory" -ForegroundColor Green
    Write-Host 'Close all terminal windows, then open a new terminal and run:'
    Write-Host '  indium --version'
    Write-Host '  indium run program.in'
}

function Uninstall-Indium([string]$Directory) {
    $Directory = Assert-IndiumInstallDirectory $Directory
    Assert-IndiumOwnedFiles $Directory
    Assert-IndiumInstallMarker $Directory
    $bin = Join-Path $Directory 'bin'
    # Never follow links or delete a file named by an editable installation record.
    foreach ($relative in $script:IndiumOwnedFiles) {
        if ($relative -eq 'install.json') { continue }
        $file = Join-Path $Directory $relative
        if (Test-Path -LiteralPath $file -PathType Leaf) { Remove-Item -LiteralPath $file -Force }
    }
    Update-IndiumUserPath $bin -Remove
    Unregister-IndiumUninstaller $Directory
    Send-IndiumEnvironmentChange
    Remove-Item -LiteralPath (Join-Path $Directory 'install.json') -Force
    foreach ($folder in @($bin, $Directory)) {
        if ((Test-Path -LiteralPath $folder -PathType Container) -and
            @(Get-ChildItem -LiteralPath $folder -Force).Count -eq 0) {
            Remove-Item -LiteralPath $folder
        }
    }
    Write-Host 'Removed the installed Indium runtime and its PATH entry.'
    Write-Host 'Your project folders and .in programs have been kept.'
}

# Dot-sourcing exposes helpers for isolated tests without installing anything.
if ($MyInvocation.InvocationName -eq '.') { return }

try {
    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) { throw 'This installer runs on Windows.' }
    if (-not $InstallDir) {
        if ($PSScriptRoot -and (Test-Path -LiteralPath (Join-Path $PSScriptRoot 'install.json'))) { $InstallDir = $PSScriptRoot }
        else { $InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\Indium' }
    }
    if ($Uninstall) {
        Uninstall-Indium $InstallDir
    } else {
        if (-not $PackageRoot) { $PackageRoot = Split-Path -Parent $PSScriptRoot }
        if (-not $InstallerText) { $InstallerText = [IO.File]::ReadAllText($PSCommandPath) }
        $source = Find-IndiumSourceDirectory $PackageRoot $SourceDir
        Install-Indium $source $InstallDir $InstallerText
    }
} catch {
    Write-Error $_ -ErrorAction Continue
    exit 1
}
