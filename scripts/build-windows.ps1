$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$sdkLib = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\Lib' -Directory -ErrorAction Stop |
  Sort-Object Name -Descending |
  ForEach-Object { Join-Path $_.FullName 'um\x64' } |
  Where-Object { Test-Path (Join-Path $_ 'DbgHelp.Lib') } |
  Select-Object -First 1
if (-not $sdkLib) { throw 'Windows SDK com DbgHelp.Lib não encontrado' }
$env:LIB = if ($env:LIB) { "$sdkLib;$env:LIB" } else { $sdkLib }
Push-Location $root
try {
  npm --prefix frontend-svelte run build
  if ($LASTEXITCODE) { throw 'Build Svelte falhou' }
  # Usa a CLI Rust instalada para evitar depender de um binário npm transitório.
  cargo tauri build --config src-tauri/tauri.conf.json
  if ($LASTEXITCODE) { throw 'Empacotamento Tauri falhou' }
} finally { Pop-Location }
