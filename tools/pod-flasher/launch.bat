@echo off
cd /d "%~dp0"
where python >nul 2>nul
if %errorlevel%==0 (
    python pod_flasher.py
) else (
    py -3 pod_flasher.py
)
pause
