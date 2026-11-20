[CmdletBinding()]
param(
    [ValidateSet('choose','install','repair','remove')][string]$Action='choose',
    [string]$PackagePath,
    [switch]$LocalTest,
    [ValidatePattern('^[A-Fa-f0-9]{64}$')][string]$ExpectedSha256
)
$ErrorActionPreference='Stop'
try {
    if (!$PackagePath) {
        $packages=@(Get-ChildItem -LiteralPath $PSScriptRoot -Filter '*.msi' -File)
        if ($packages.Count -ne 1) {throw 'Debe existir exactamente un MSI junto al asistente.'}
        $PackagePath=$packages[0].FullName
    }
    $PackagePath=(Resolve-Path -LiteralPath $PackagePath).Path
    if ($Action -eq 'choose') {
        Add-Type -AssemblyName System.Windows.Forms
        Add-Type -AssemblyName System.Drawing
        $form=New-Object Windows.Forms.Form
        $form.Text='Monitor — Agente del equipo';$form.ClientSize=New-Object Drawing.Size(520,230)
        $form.StartPosition='CenterScreen';$form.FormBorderStyle='FixedDialog';$form.MaximizeBox=$false
        $label=New-Object Windows.Forms.Label
        $label.SetBounds(20,20,480,95)
        $label.Text="Instala, actualiza o repara el agente de este equipo.`r`nLa desinstalación conserva configuración, identidad y datos pendientes.`r`nSe solicitarán permisos de administrador. El servidor y el enrolamiento se configuran por separado."
        if ($LocalTest) {$label.Text+="`r`nPILOTO LOCAL: paquete sin firma, comprobado mediante SHA256."}
        $choice=New-Object Windows.Forms.ComboBox
        $choice.SetBounds(20,120,480,30);$choice.DropDownStyle='DropDownList'
        $null=$choice.Items.AddRange(@('Instalar o actualizar','Reparar','Desinstalar'));$choice.SelectedIndex=0
        $button=New-Object Windows.Forms.Button
        $button.Text='Continuar';$button.SetBounds(370,175,130,35);$button.DialogResult='OK'
        $form.Controls.AddRange(@($label,$choice,$button));$form.AcceptButton=$button
        if ($form.ShowDialog() -ne 'OK') {exit 0}
        $Action=@('install','repair','remove')[$choice.SelectedIndex]
        $form.Dispose()
    }
    $principal=[Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
    if (!$principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        # Quote PowerShell literals before encoding; paths are never executable text.
        $command="& '"+$PSCommandPath.Replace("'","''")+"' -Action '"+$Action+"' -PackagePath '"+$PackagePath.Replace("'","''")+"'"
        if ($LocalTest) {
            if (!$ExpectedSha256) {throw 'El piloto local requiere un SHA256 explícito.'}
            $command+=" -LocalTest -ExpectedSha256 '$ExpectedSha256'"
        }
        $encoded=[Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($command))
        $child=Start-Process powershell.exe -Verb RunAs -WindowStyle Hidden -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-EncodedCommand',$encoded) -Wait -PassThru
        exit $child.ExitCode
    }
    $installArguments=@{PackagePath=$PackagePath;Action=$Action}
    $previousService=Get-Service ActivityMonitor -ErrorAction SilentlyContinue
    $resumeExisting=($previousService -and $previousService.Status -eq 'Running')
    if ($LocalTest) {
        if (!$ExpectedSha256) {throw 'El piloto local requiere un SHA256 explícito.'}
        $installArguments.LocalTest=$true;$installArguments.ExpectedSha256=$ExpectedSha256
    }
    & (Join-Path $PSScriptRoot 'install-package.ps1') @installArguments
    $message='Operación completada. La configuración y los datos se conservaron.'
    if ($Action -ne 'remove' -and $resumeExisting) {
        Start-Service ActivityMonitor
        (Get-Service ActivityMonitor).WaitForStatus('Running',[TimeSpan]::FromSeconds(30))
        Get-ScheduledTask -TaskName 'ActivityMonitor-Session-*' -TaskPath '\' -ErrorAction SilentlyContinue |
            Where-Object {$_.Settings.Enabled} | ForEach-Object {Start-ScheduledTask -TaskName $_.TaskName -TaskPath $_.TaskPath}
        $message+=' El servicio se reanudó y se solicitaron las sesiones previamente habilitadas.'
    } elseif ($Action -ne 'remove') {
        $message+=' Antes de iniciar el servicio, revisa el enrolamiento y la configuración con el administrador.'
    }
    Add-Type -AssemblyName System.Windows.Forms
    $null=[Windows.Forms.MessageBox]::Show($message,'Monitor')
} catch {
    Add-Type -AssemblyName System.Windows.Forms
    $null=[Windows.Forms.MessageBox]::Show($_.Exception.Message,'Monitor — Operación no completada','OK','Error')
    exit 1
}
