@echo off
cd /d "%~dp0"
npm run tauri:dev
if errorlevel 1 pause
