param([Parameter(Mandatory=$true)][string]$Backup)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$backupPath = (Resolve-Path -LiteralPath $Backup).Path
if ([IO.Path]::GetExtension($backupPath) -ne '.dump') { throw 'Informe um arquivo .dump válido' }
$container = (docker compose -f (Join-Path $root 'compose.yaml') ps -q postgres).Trim()
if (-not $container) { throw 'Container PostgreSQL não está em execução' }
$remoteDump = "/tmp/commercectrl-restore-$([guid]::NewGuid().ToString('N')).dump"
docker cp $backupPath "${container}:${remoteDump}"
docker compose -f (Join-Path $root 'compose.yaml') exec -T postgres pg_restore -U commercectrl -d commercectrl --clean --if-exists --no-owner $remoteDump
if ($LASTEXITCODE) { throw 'Restauração PostgreSQL falhou' }
docker compose -f (Join-Path $root 'compose.yaml') exec -T postgres rm -f $remoteDump
docker compose -f (Join-Path $root 'compose.yaml') restart backend
