# Shared administrative session helpers. Dot-source only from an administrator-controlled package.
function Assert-NoReparse([string]$Path) {
    $item = Get-Item -LiteralPath $Path -Force
    while ($null -ne $item) {
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Reparse path rejected: $($item.FullName)" }
        $item = if ($item -is [IO.DirectoryInfo]) { $item.Parent } else { $item.Directory }
    }
}
function Assert-AdminControlled([string]$Path) {
    $acl = Get-Acl -LiteralPath $Path
    $currentSid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    $trusted = @($adminSid.Value, $systemSid.Value, $currentSid)
    if ($acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -notin $trusted) { throw "Untrusted owner: $Path" }
    $writeMask = [Security.AccessControl.FileSystemRights]::Write -bor [Security.AccessControl.FileSystemRights]::Delete -bor [Security.AccessControl.FileSystemRights]::DeleteSubdirectoriesAndFiles -bor [Security.AccessControl.FileSystemRights]::ChangePermissions -bor [Security.AccessControl.FileSystemRights]::TakeOwnership
    foreach ($rule in $acl.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier])) {
        if ($rule.AccessControlType -eq 'Allow' -and ($rule.FileSystemRights -band $writeMask) -and $rule.IdentityReference.Value -notin $trusted) {
            throw "Non-administrative write access must be removed before enrollment: $Path"
        }
    }
}
function Protect-Directory([string]$Path, [bool]$AllowSession) {
    if (-not (Test-Path -LiteralPath $Path)) { New-Item -ItemType Directory -Path $Path | Out-Null }
    Assert-NoReparse $Path
    $acl = [System.Security.AccessControl.DirectorySecurity]::new()
    $acl.SetAccessRuleProtection($true, $false)
    $acl.SetOwner($adminSid)
    foreach ($sid in @($adminSid, $systemSid)) {
        $acl.AddAccessRule([System.Security.AccessControl.FileSystemAccessRule]::new($sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
    }
    if ($AllowSession) {
        $acl.AddAccessRule([System.Security.AccessControl.FileSystemAccessRule]::new($userSid, 'ReadAndExecute', 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
    }
    Set-Acl -LiteralPath $Path -AclObject $acl
}
function Write-Protected([string]$Path, [string]$Content, [bool]$AllowSession) {
    if (Test-Path -LiteralPath $Path) { Assert-NoReparse $Path }
    $temporary = Join-Path $enrollmentPath ('.enroll-' + [guid]::NewGuid().ToString() + '.tmp')
    try {
        [IO.File]::WriteAllText($temporary, $Content, $utf8)
        $acl = [System.Security.AccessControl.FileSecurity]::new()
        $acl.SetAccessRuleProtection($true, $false)
        $acl.SetOwner($adminSid)
        foreach ($sid in @($adminSid, $systemSid)) {
            $acl.AddAccessRule([System.Security.AccessControl.FileSystemAccessRule]::new($sid, 'FullControl', 'Allow'))
        }
        if ($AllowSession) { $acl.AddAccessRule([System.Security.AccessControl.FileSystemAccessRule]::new($userSid, 'Read', 'Allow')) }
        Set-Acl -LiteralPath $temporary -AclObject $acl
        $durable = [IO.File]::Open($temporary, [IO.FileMode]::Open, [IO.FileAccess]::Write, [IO.FileShare]::None)
        try { $durable.Flush($true) } finally { $durable.Dispose() }
        if (Test-Path -LiteralPath $Path) {
            Set-Acl -LiteralPath $Path -AclObject $acl
            [IO.File]::Replace($temporary, $Path, [System.Management.Automation.Language.NullString]::Value)
        }
        else { [IO.File]::Move($temporary, $Path) }
        # Replace can preserve destination metadata; enforce the intended final ACL too.
        Set-Acl -LiteralPath $Path -AclObject $acl
    } finally {
        if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Force }
    }
}

function Get-SessionSetting([string]$Text, [string]$Name, [bool]$Required = $true) {
    $values = [regex]::Matches($Text, '(?m)^\s*(?:export\s+)?' + [regex]::Escape($Name) + '\s*=([^\r\n]*)$')
    if ($values.Count -gt 1 -or ($Required -and $values.Count -eq 0)) { throw "Missing or ambiguous setting: $Name" }
    if ($values.Count -eq 0) { return $null }
    return $values[0].Groups[1].Value.Trim().Trim([char]34, [char]39)
}
function Update-SessionEnvironment([string]$Text, $Settings) {
    $pattern = '^\s*(?:export\s+)?(' + (($Settings.Keys | ForEach-Object { [regex]::Escape($_) }) -join '|') + ')\s*='
    $lines = @($Text -split '\r?\n' | Where-Object { $_ -notmatch $pattern })
    $updated = ($lines -join "`n") + "`n"
    foreach ($entry in $Settings.GetEnumerator()) { $updated += "$($entry.Key)=$($entry.Value)`n" }
    return $updated
}
function New-SessionSecret {
    $bytes = New-Object byte[] 32
    $rng = [Security.Cryptography.RandomNumberGenerator]::Create()
    try { $rng.GetBytes($bytes) } finally { $rng.Dispose() }
    return [BitConverter]::ToString($bytes).Replace('-', '').ToLowerInvariant()
}
