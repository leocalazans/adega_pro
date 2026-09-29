param([string]$Destination = (Join-Path $PSScriptRoot '..\backups'))
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$destinationPath = [IO.Path]::GetFullPath($Destination)
New-Item -ItemType Directory -Force -Path $destinationPath | Out-Null
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$dump = Join-Path $destinationPath "commercectrl-postgres-$stamp.dump"
$container = (docker compose -f (Join-Path $root 'compose.yaml') ps -q postgres).Trim()
if (-not $container) { throw 'Container PostgreSQL não está em execução' }
$remoteDump = "/tmp/commercectrl-$stamp.dump"
docker compose -f (Join-Path $root 'compose.yaml') exec -T postgres pg_dump -U commercectrl -d commercectrl -Fc -f $remoteDump
if ($LASTEXITCODE) { throw 'pg_dump falhou' }
docker cp "${container}:${remoteDump}" $dump
docker compose -f (Join-Path $root 'compose.yaml') exec -T postgres rm -f $remoteDump
if (-not (Test-Path $dump) -or (Get-Item $dump).Length -lt 1024) { throw 'Backup PostgreSQL inválido' }
$hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $dump).Hash
Set-Content -LiteralPath "$dump.sha256" -Value "$hash  $([IO.Path]::GetFileName($dump))"
Write-Output $dump
