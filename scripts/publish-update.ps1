param([Parameter(Mandatory=$true)][string]$Version)
$ErrorActionPreference='Stop'
foreach($name in 'SUPABASE_URL','SUPABASE_SERVICE_ROLE_KEY','SUPABASE_UPDATE_BUCKET') { if(-not (Get-Item "env:$name" -ErrorAction SilentlyContinue).Value){throw "Variável $name ausente"} }
$bundle=Join-Path $PSScriptRoot '..\src-tauri\target\release\bundle'
$installer=Get-ChildItem $bundle -Recurse -File | Where-Object {$_.Name -match '\.(nsis|msi)\.zip$'} | Sort-Object Length -Descending | Select-Object -First 1
if(-not $installer){throw 'Artefato de atualização Tauri (.nsis.zip/.msi.zip) não encontrado'}
$signaturePath="$($installer.FullName).sig"
if(-not (Test-Path -LiteralPath $signaturePath)){throw "Assinatura não encontrada: $signaturePath"}
$object="CommerceCTRL-$Version-$($installer.Name)"
$headers=@{Authorization="Bearer $env:SUPABASE_SERVICE_ROLE_KEY";apikey=$env:SUPABASE_SERVICE_ROLE_KEY;'x-upsert'='true'}
$upload="$env:SUPABASE_URL/storage/v1/object/$env:SUPABASE_UPDATE_BUCKET/$object"
Invoke-RestMethod -Method Post -Uri $upload -Headers $headers -ContentType 'application/octet-stream' -InFile $installer.FullName | Out-Null
$publicUrl="$env:SUPABASE_URL/storage/v1/object/public/$env:SUPABASE_UPDATE_BUCKET/$object"
$manifest=@{version=$Version;notes='Atualização CommerceCTRL';pub_date=(Get-Date).ToUniversalTime().ToString('o');platforms=@{'windows-x86_64'=@{signature=(Get-Content -LiteralPath $signaturePath -Raw).Trim();url=$publicUrl}}}|ConvertTo-Json -Depth 8
$temp=Join-Path ([IO.Path]::GetTempPath()) "commercectrl-latest-$([guid]::NewGuid().ToString('N')).json"
Set-Content -LiteralPath $temp -Value $manifest -Encoding utf8
Invoke-RestMethod -Method Post -Uri "$env:SUPABASE_URL/storage/v1/object/$env:SUPABASE_UPDATE_BUCKET/latest.json" -Headers $headers -ContentType 'application/json' -InFile $temp | Out-Null
Remove-Item -LiteralPath $temp -Force
Write-Output "Publicado $publicUrl"
