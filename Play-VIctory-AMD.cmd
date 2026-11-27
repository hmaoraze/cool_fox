@echo off
setlocal EnableExtensions DisableDelayedExpansion
rem Radeon cards (RX 7000+, 16 GB+ VRAM): recorded command lists and all four
rem renderer ports, which is what the leading community forks run with.
rem
rem The table lives in apps/victory/src/profile.rs. `--vendor amd` is what makes
rem `balanced` and `speed` pick the AMD switches; the default is the safe set.
rem
rem Another set as the first argument:
rem   Play-VIctory-AMD.cmd balanced | speed | quality | lowvram
set "PROFILE=%~1"
if not defined PROFILE set "PROFILE=pareto-residency-amd"
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
"%runner%" run "%VICTORY_GAME_DIR%\game_scarlett_master.exe" --profile "%PROFILE%" --vendor amd --execute
exit /b %errorlevel%
