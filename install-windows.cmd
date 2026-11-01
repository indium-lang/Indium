@echo off
setlocal
powershell.exe -NoLogo -NoProfile -STA -ExecutionPolicy Bypass -File "%~dp0scripts\install-windows.ps1" -PackageRoot "%~dp0." %*
set "indiumSetupExit=%ERRORLEVEL%"
echo(
pause
exit /b %indiumSetupExit%
