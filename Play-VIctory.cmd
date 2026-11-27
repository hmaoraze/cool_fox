@echo off
setlocal EnableExtensions DisableDelayedExpansion
rem Safe default for every GPU: Series X, no recorded lists, renderer ports 6
rem and 7 only, the constant ambient stand-in.
rem
rem The table itself lives in apps/victory/src/profile.rs now; this file only
rem names a set, and the binary validates the name. `victory profiles` prints
rem every table, `victory gpu-info` prints the card a run would land on.
rem
rem Another set as the first argument:
rem   Play-VIctory.cmd balanced | speed | quality | lowvram
rem   Play-VIctory.cmd pareto-residency-amd          (Radeon: all four ports)
set "PROFILE=%~1"
if not defined PROFILE set "PROFILE=pareto-residency"
if not defined VICTORY_GAME_DIR (
  echo Set VICTORY_GAME_DIR to the folder containing your title executable. 1>&2
  exit /b 2
)
if not exist "%VICTORY_GAME_DIR%\game_scarlett_master.exe" (
  echo game_scarlett_master.exe not found in VICTORY_GAME_DIR: "%VICTORY_GAME_DIR%" 1>&2
  exit /b 2
)
set "runner=%~dp0victory-3d-rtx-pareto-residency.exe"
if not exist "%runner%" set "runner=%~dp0target\release\victory.exe"
if not exist "%runner%" (
  echo Extract the Windows release or run cargo build --release --locked -p victory. 1>&2
  exit /b 2
)
rem A switch already in the environment wins over the preset, as `if not
rem defined` did: `set VICTORY_AMBIENT=0.4` still changes one value only.
"%runner%" run "%VICTORY_GAME_DIR%\game_scarlett_master.exe" --profile "%PROFILE%" --execute
exit /b %errorlevel%
