@echo off
setlocal DisableDelayedExpansion
cd /d D:\global\CommerceCTRL
echo.
echo Gerando o par Ed25519 de licenciamento localmente.
echo Nao envie nem versione LICENSE_SIGNING_KEY_B64.
echo.
cargo run --manifest-path backend\Cargo.toml --bin license-keygen
echo.
echo Copie somente a linha LICENSE_SIGNING_KEY_B64 para o Render.
echo A chave publica sera obtida automaticamente pelo processo de release.
pause
