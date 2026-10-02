@echo off
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
cargo --version
call npx tauri build
