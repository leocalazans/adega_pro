param([switch]$SendPrinterTest)
$ErrorActionPreference='Stop'
$printers=Get-CimInstance Win32_Printer | Select-Object Name,Default,Network,PrinterStatus,WorkOffline
if(-not $printers){throw 'Nenhuma impressora instalada no Windows'}
$printers | Format-Table -AutoSize
if($SendPrinterTest){
  $default=$printers|Where-Object Default|Select-Object -First 1
  if(-not $default){throw 'Defina uma impressora padrão no Windows'}
  "CommerceCTRL`r`nTeste gráfico de impressão`r`n$(Get-Date)"|Out-Printer -Name $default.Name
  Write-Host "Teste enviado para $($default.Name). Confirme visualmente o papel."
}
Write-Host 'Leitor: abra o Bloco de Notas, bipe um produto e confirme que o código termina com ENTER.'
Write-Host 'PDV: no aplicativo, bipe o mesmo código sem focar um campo e confirme que o produto entra no carrinho.'
Write-Host 'Offline: desconecte a rede, faça uma venda, reconecte e confirme que Pendentes volta a zero.'
Write-Host 'Atualização: publique uma versão maior no canal de homologação e use Configurações > Buscar atualização.'
