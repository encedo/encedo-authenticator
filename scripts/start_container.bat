:: starts a Docker container for the project
set SCRIPT_DIR=%~dp0

docker run -it --rm -v "%SCRIPT_DIR%\..:/app" beevelop/ionic bash