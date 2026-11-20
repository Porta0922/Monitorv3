[CmdletBinding()]
param(
    [Parameter(Mandatory)][string[]]$Artifact,
    [Parameter(Mandatory)][ValidatePattern('^[A-Fa-f0-9]{40}$')][string]$CertificateThumbprint,
    [Parameter(Mandatory)][uri]$TimestampUrl,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [ValidateSet('CurrentUser','LocalMachine')][string]$CertificateStore='CurrentUser',
    [string]$SignTool='signtool.exe'
)
$ErrorActionPreference='Stop'
if (!$TimestampUrl.IsAbsoluteUri -or $TimestampUrl.Scheme -ne 'https' -or $TimestampUrl.UserInfo -or $TimestampUrl.Fragment -or $TimestampUrl.Query) {
    throw 'Use an absolute HTTPS timestamp URL without credentials or fragment.'
}
$certificate=Get-Item -LiteralPath "Cert:\$CertificateStore\My\$CertificateThumbprint"
if (!$certificate.HasPrivateKey -or $certificate.NotAfter -le (Get-Date) -or $certificate.NotBefore -gt (Get-Date)) {
    throw 'A currently valid organization certificate with its private key is required.'
}
$tool=(Get-Command $SignTool -CommandType Application -ErrorAction Stop).Source
$inputs=@();$names=@{}
foreach ($path in $Artifact) {
    $item=Get-Item -LiteralPath $path
    if ($item.PSIsContainer -or $item.Extension -notin @('.exe','.msi','.dll') -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw 'Only regular EXE, MSI or DLL release artifacts are supported.'
    }
    if ($names.ContainsKey($item.Name)) {throw 'Artifact basenames must be unique.'}
    $names[$item.Name]=$true;$inputs+=$item
}
$destination=[IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $destination) {throw 'Use a new output directory; existing releases are never overwritten.'}
New-Item -ItemType Directory -Path $destination | Out-Null
$pending=Join-Path $destination 'SIGNING-INCOMPLETE.txt'
Set-Content -LiteralPath $pending -Value 'Do not distribute: signing has not completed for every artifact.'
$receipts=@()
foreach ($item in $inputs) {
    $signed=Join-Path $destination $item.Name
    Copy-Item -LiteralPath $item.FullName -Destination $signed
    $arguments=@('sign','/fd','SHA256','/td','SHA256','/tr',$TimestampUrl.AbsoluteUri,'/sha1',$CertificateThumbprint,'/s','My')
    if ($CertificateStore -eq 'LocalMachine') {$arguments+='/sm'}
    & $tool @arguments $signed
    if ($LASTEXITCODE -ne 0) {throw 'Signing failed; the incomplete marker is retained.'}
    & $tool verify /pa /all /tw $signed
    if ($LASTEXITCODE -ne 0) {throw 'Signature verification failed; do not distribute output.'}
    $signature=Get-AuthenticodeSignature -LiteralPath $signed
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Thumbprint -ne $CertificateThumbprint -or !$signature.TimeStamperCertificate) {
        throw 'Publisher or trusted timestamp verification failed.'
    }
    $receipts+=@{name=$item.Name;sha256=(Get-FileHash -LiteralPath $signed -Algorithm SHA256).Hash;publisher=$CertificateThumbprint.ToUpperInvariant();timestamped=$true}
}
@{schema_version=1;artifacts=$receipts} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $destination 'signing-receipt.json') -Encoding utf8
Remove-Item -LiteralPath $pending
Write-Output 'Signing completed. Generate the release manifest from these final bytes; no artifacts were published.'
