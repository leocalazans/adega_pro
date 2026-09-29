$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$sdkLib = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\Lib' -Directory -ErrorAction SilentlyContinue |
  Sort-Object Name -Descending |
  ForEach-Object { Join-Path $_.FullName 'um\x64' } |
  Where-Object { Test-Path (Join-Path $_ 'DbgHelp.Lib') } |
  Select-Object -First 1
if ($sdkLib) { $env:LIB = if ($env:LIB) { "$sdkLib;$env:LIB" } else { $sdkLib } }
Push-Location $root
try {
  npm --prefix frontend-svelte run test:coverage
  if ($LASTEXITCODE) { throw 'Testes Svelte falharam' }
  npm --prefix frontend-svelte run check
  if ($LASTEXITCODE) { throw 'svelte-check falhou' }
  npm --prefix frontend-svelte run build
  if ($LASTEXITCODE) { throw 'Build Svelte falhou' }
  docker run --rm -v "${root}:/work" -w /work/core-tests rust:1.90-bookworm cargo test --locked
  if ($LASTEXITCODE) { throw 'Testes SQLite falharam' }
  docker compose up -d --build
  if ($LASTEXITCODE) { throw 'Build dos containers falhou' }
  & "$root/backend/tests/integration.ps1"
  if ($LASTEXITCODE) { throw 'Integração backend falhou' }
  cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
  if ($LASTEXITCODE) { throw 'Formatação Tauri falhou' }
  cargo fmt --manifest-path backend/Cargo.toml --all -- --check
  if ($LASTEXITCODE) { throw 'Formatação backend falhou' }
  if ($sdkLib) {
    cargo test --manifest-path src-tauri/Cargo.toml
    if ($LASTEXITCODE) { throw 'Testes unitários Tauri falharam' }
    cargo test --manifest-path backend/Cargo.toml
    if ($LASTEXITCODE) { throw 'Testes unitários backend falharam' }
    cargo check --manifest-path src-tauri/Cargo.toml
    if ($LASTEXITCODE) { throw 'Compilação Windows/Tauri falhou' }
  }
  Write-Host 'Todos os portões automatizados passaram.' -ForegroundColor Green
} finally {
  Pop-Location
}
