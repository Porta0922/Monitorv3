[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$BinaryPath,
    [Parameter(Mandatory)][ValidatePattern('^[a-fA-F0-9]{64}$')][string]$ExpectedSha256,
    [Parameter(Mandatory)][ValidatePattern('^[0-9]+\.[0-9]+\.[0-9]+$')][string]$Version,
    [Parameter(Mandatory)][string]$OutputPath,
    [string]$ConfigTemplate
)
$ErrorActionPreference='Stop'
$artifact=(Resolve-Path -LiteralPath $BinaryPath).Path
$destination=[IO.Path]::GetFullPath($OutputPath)
$outputDirectory=Split-Path -Parent $destination
New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
$stage=Join-Path $outputDirectory ('.build-msi-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $stage | Out-Null
# Retain the build inputs for diagnosis; never recursively delete a computed directory.
Copy-Item -LiteralPath $artifact -Destination (Join-Path $stage 'activity-monitor-agent.exe')
if ((Get-FileHash -LiteralPath (Join-Path $stage 'activity-monitor-agent.exe') -Algorithm SHA256).Hash -ne $ExpectedSha256) { throw 'Artifact checksum mismatch.' }
if (!$ConfigTemplate) {$ConfigTemplate=Join-Path (Split-Path -Parent $PSScriptRoot) 'agent.env.example'}
Copy-Item -LiteralPath (Resolve-Path -LiteralPath $ConfigTemplate).Path -Destination (Join-Path $stage 'agent.env')
[IO.File]::WriteAllText((Join-Path $stage 'Mi equipo.url'),"[InternetShortcut]`r`nURL=http://127.0.0.1:9876/`r`n",[Text.UTF8Encoding]::new($false))
$namespace='http://wixtoolset.org/schemas/v4/wxs'
$xml=[Xml.XmlDocument]::new()
$wix=$xml.CreateElement('Wix',$namespace);$null=$xml.AppendChild($wix)
$fragment=$xml.CreateElement('Fragment',$namespace);$null=$wix.AppendChild($fragment)
$group=$xml.CreateElement('ComponentGroup',$namespace);$group.SetAttribute('Id','MaintenanceScripts');$group.SetAttribute('Directory','MAINTENANCE');$null=$fragment.AppendChild($group)
$packageScripts=@(Get-ChildItem -LiteralPath $PSScriptRoot -Filter '*.ps1' -File | Where-Object { $_.Name -ne 'build-msi.ps1' })
$packageScripts+=Get-Item -LiteralPath (Join-Path (Split-Path -Parent $PSScriptRoot) 'common\data-lifecycle.py')
foreach ($script in ($packageScripts | Sort-Object Name)) {
    $id='Script_' + ($script.BaseName -replace '[^a-zA-Z0-9_]','_')
    $component=$xml.CreateElement('Component',$namespace);$component.SetAttribute('Id',$id);$component.SetAttribute('Guid','*')
    $file=$xml.CreateElement('File',$namespace);$file.SetAttribute('Id',$id+'_file');$file.SetAttribute('Source',$script.FullName);$file.SetAttribute('KeyPath','yes')
    $null=$component.AppendChild($file);$null=$group.AppendChild($component)
}
$scripts=Join-Path $stage 'Scripts.wxs';$xml.Save($scripts)
& wix build (Join-Path $PSScriptRoot 'msi\Package.wxs') $scripts -arch x64 -ext WixToolset.Util.wixext -d "Payload=$stage" -d "Version=$Version" -o $destination
if ($LASTEXITCODE -ne 0) { throw 'MSI build failed.' }
Write-Output "Unsigned MSI generated: $destination. Sign before distribution."
