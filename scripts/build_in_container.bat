:: Bat file that starts a Docker build for the project
@echo off

set SCRIPT_DIR=%~dp0

docker run -it --rm -v "%SCRIPT_DIR%\..:/app" beevelop/ionic bash /app/scripts/build.sh