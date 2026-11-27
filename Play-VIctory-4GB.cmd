@echo off
setlocal EnableExtensions DisableDelayedExpansion
rem 4 GB cards: 720p on the Series S profile, coarser card-buffer granularity,
rem the driver's background recompile threads off.
rem
rem The table lives in apps/victory/src/profile.rs. Series S can drop head and
rem skin meshes on some cards; that is the known trade, not a fault.
set "PROFILE=%~1"
if not defined PROFILE set "PROFILE=lowvram"
if not defined VICTORY_GAME_DIR (
  echo Set VICTORY_GAME_DIR to the folder containing your title executable. 1>&2
  exit /b 2
)
if not exist "%VICTORY_GAME_DIR%\game_scarlett_master.exe" (
  echo game_scarlett_master.exe not found in VICTORY_GAME_DIR: "%VICTORY_GAME_DIR%" 1>&2
  exit /b 2
)
set "runner=%~dp0target\release\victory.exe"
if not exist "%runner%" set "runner=%~dp0victory-3d-rtx-pareto-residency.exe"
if not exist "%runner%" (
  echo Extract the Windows release or run cargo build --release --locked -p victory. 1>&2
  exit /b 2
)
"%runner%" run "%VICTORY_GAME_DIR%\game_scarlett_master.exe" --profile "%PROFILE%" --execute
exit /b %errorlevel%
