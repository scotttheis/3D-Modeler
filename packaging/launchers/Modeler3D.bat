@echo off
setlocal
set "APP_DIR=%~dp0"
set "APP_EXE=%APP_DIR%Modeler3D.exe"
if not exist "%APP_EXE%" (
    >&2 echo Modeler3D.exe was not found beside this launcher.
    exit /b 2
)
start "3D Modeler" "%APP_EXE%" %*
exit /b 0
