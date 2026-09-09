@echo off

if /I "%~1"=="auth" if /I "%~2"=="status" goto auth_status
if /I "%~1"=="--ax-screen-reader" goto interactive

>claude-fixture-unsupported.marker echo started
>&2 echo unsupported fixture arguments
exit /b 2

:auth_status
>claude-fixture-auth-started.marker echo started
echo Logged in
exit /b 0

:interactive
>claude-fixture-interactive-started.marker echo started
echo Claude Code ready

:read_input
set "ai_meter_input="
set /p "ai_meter_input="
if /I "%ai_meter_input%"=="/usage" goto usage
goto read_input

:usage
>claude-fixture-usage-received.marker echo received
echo Current session
echo 23%% used
echo Resets in 3 hr 42 min
echo Current week (all models)
echo 5%% used
echo Resets Sep 6 at 8:00am
goto read_input
