@echo off
rem Installs a Python module for whoever runs this, into
rem %APPDATA%\collomatique\python -- their own, writable without administrator
rem rights, and read back by Collomatique through collomatique_site.py.
rem
rem The prefix is asked of that module rather than written here, so the Python
rem version in the path stays in one place.
rem
rem Double-clicking asks for a module name; passing it on the command line works
rem too. "pause" at the end so that a window opened by double-click stays long
rem enough to read what pip said.
setlocal

rem The interpreter is the one beside this file, and it is reached through the
rem current directory rather than by its full path. "for /f" runs its command
rem through cmd /c, which mangles a command line that both starts with a quote
rem and contains more -- and the full path does start with one as soon as it
rem holds a space, which "C:\Program Files\Collomatique" does. Starting the
rem command with ".\" avoids that rule entirely.
pushd "%~dp0"

for /f "delims=" %%p in ('.\python.exe -c "import collomatique_site; print(collomatique_site.prefix())"') do set "COLLO_PREFIX=%%p"

set "COLLO_MODULE=%*"
if not defined COLLO_MODULE set /p "COLLO_MODULE=Nom du module a installer : "

rem --no-warn-script-location: the Scripts directory of that prefix is not on
rem PATH and is not meant to be. Only imports matter here.
.\python.exe -m pip install --prefix "%COLLO_PREFIX%" --no-warn-script-location %COLLO_MODULE%

popd
echo.
pause
