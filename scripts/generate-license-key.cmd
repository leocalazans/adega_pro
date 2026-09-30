@echo off
setlocal DisableDelayedExpansion
cd /d D:\global\CommerceCTRL
set "WINDOWS_SDK_LIB="
for /d %%D in ("C:\Program Files (x86)\Windows Kits\10\Lib\*") do (
  if exist "%%~fD\um\x64\dbghelp.lib" set "WINDOWS_SDK_LIB=%%~fD\um\x64"
)
if "%WINDOWS_SDK_LIB%"=="" (
  echo Nao foi encontrado dbghelp.lib no Windows SDK x64.
  echo Instale o componente Windows 10/11 SDK pelo Visual Studio Installer.
  pause
  exit /b 1
)
set "LIB=%WINDOWS_SDK_LIB%;%LIB%"
echo.
echo Gerando o par Ed25519 de licenciamento localmente.
echo Nao envie nem versione LICENSE_SIGNING_KEY_B64.
echo.
cargo run --manifest-path backend\Cargo.toml --bin license-keygen
echo.
echo Copie somente a linha LICENSE_SIGNING_KEY_B64 para o Render.
echo A chave publica sera obtida automaticamente pelo processo de release.
pause
