# ============================================================
#  publish.ps1 - Publica un release (AGENTE o REMOVER) via API
#  Pipeline seguro: build -> validar version embebida -> sha256
#                    -> subir asset al release correcto -> abort si no coincide
#
#  Requiere: github_token.txt en la raiz del repo (gitignoreado).
#  Uso:
#    powershell -File scripts\publish.ps1 -Role agent
#    powershell -File scripts\publish.ps1 -Role remover
# ============================================================
param(
    [ValidateSet('agent', 'remover')]
    [string]$Role = 'agent',
    [switch]$SkipBuild
)
$ErrorActionPreference = 'Stop'

$RepoRoot = Split-Path -Parent $PSScriptRoot
$TokenFile = Join-Path $RepoRoot 'github_token.txt'
$Owner = 'Porta0922'
$Repo = 'Monitorv3'
$ApiBase = "https://api.github.com/repos/$Owner/$Repo"
$AssetName = 'activity-monitor-agent.exe'

# ---------- 1. Token ----------
if (-not (Test-Path -LiteralPath $TokenFile)) {
    Write-Error "No se encuentra $TokenFile (gitignoreado). Pegar ahi el GITHUB_TOKEN."
}
$Token = (Get-Content -LiteralPath $TokenFile -Raw).Trim()
if (-not $Token) { Write-Error "Token vacio en $TokenFile." }

# ---------- 2. Version desde Cargo.toml ----------
if ($Role -eq 'agent') {
    $CrateDir = Join-Path $RepoRoot 'agent'
    $ExpectedBanner = 'Agent v'
} else {
    $CrateDir = Join-Path $RepoRoot 'remover'
    $ExpectedBanner = 'Removedor v'
}
$Cargo = Get-Content -LiteralPath (Join-Path $CrateDir 'Cargo.toml')
$Version = ($Cargo | Where-Object { $_ -match '^version\s*=\s*"([^"]+)"' } | Select-Object -First 1) -replace '^version\s*=\s*"([^"]+)"', '$1'
if (-not $Version) { Write-Error "No se pudo leer la version de $($CrateDir)\Cargo.toml" }
Write-Host "[1/6] Role=$Role  version= v$Version"

# ---------- 3. Build ----------
$Pkg = if ($Role -eq 'agent') { 'activity-monitor-agent' } else { 'activity-monitor-remover' }
if ($SkipBuild) {
    Write-Host "[2/6] SkipBuild: usando el binario existente"
} else {
    Write-Host "[2/6] cargo build --release -p $Pkg ..."
    Push-Location $RepoRoot
    try { & cargo build --release -p $Pkg 2>&1 | Out-Null } finally { Pop-Location }
    if ($LASTEXITCODE -ne 0) { Write-Error "Build fallo (cargo exit $LASTEXITCODE)." }
}
$Bin = Join-Path $RepoRoot "target\release\$AssetName"
if (-not (Test-Path -LiteralPath $Bin)) { Write-Error "No existe el binario: $Bin" }

# ---------- 4. Validar version embebida ----------
$Bytes = [System.IO.File]::ReadAllBytes($Bin)
$Ascii = [System.Text.Encoding]::ASCII.GetString($Bytes)
$Needle = "$ExpectedBanner$Version"
if (-not $Ascii.Contains($Needle)) {
    Write-Error "ABORT: el binario no contiene '$Needle'. Posible binario del rol equivocado. No se publica nada."
}
$Sha = (Get-FileHash -LiteralPath $Bin -Algorithm SHA256).Hash.ToLower()
Write-Host "[3/6] Validado: contiene '$Needle'"
Write-Host "[4/6] SHA256: $Sha  ($($(Get-Item $Bin).Length) bytes)"

if ($Role -eq 'remover') {
    Write-Warning "ATENCION: publicar REMOVER lo convierte en 'latest' -> la flota se auto-desinstala."
    Write-Host "Para volver a 'latest'=agente, publica despues el agente (publish.ps1 -Role agent)."
}

# ---------- 5. Release / asset existente ----------
$Headers = @{ Authorization = "Bearer $Token"; Accept = 'application/vnd.github+json'; 'User-Agent' = 'manager-publisher' }
$Tag = "v$Version"
Write-Host "[5/6] Tag=$Tag"
$Release = $null
try {
    $Release = Invoke-RestMethod -Method Get -Uri "$ApiBase/releases/tags/$Tag" -Headers $Headers
} catch { $Release = $null }

if (-not $Release) {
    Write-Host "   Release $Tag no existe. Creando..."
    $Body = @{ tag_name = $Tag; target_commitish = 'main'; name = "v$Version";
               body = "Publicado por scripts\publish.ps1 (role=$Role). SHA256: $Sha";
               draft = $false; prerelease = $false } | ConvertTo-Json
    $Release = Invoke-RestMethod -Method Post -Uri "$ApiBase/releases" -Headers $Headers -Body $Body -ContentType 'application/json'
}
Write-Host "   Release id=$($Release.id)  html=$($Release.html_url)"

$Existing = $Release.assets | Where-Object { $_.name -eq $AssetName }
foreach ($a in $Existing) {
    Write-Host "   Reemplazando asset previo id=$($a.id) size=$($a.size)"
    Invoke-RestMethod -Method Delete -Uri "$ApiBase/releases/assets/$($a.id)" -Headers $Headers | Out-Null
}

# ---------- 6. Upload ----------
Write-Host "[6/6] Subiendo asset..."
$UpUrl = "https://uploads.github.com/repos/$Owner/$Repo/releases/$($Release.id)/assets?name=$AssetName"
$Up = & curl.exe -sS -X POST -H "Authorization: Bearer $Token" -H "Content-Type: application/octet-stream" `
        -H "Accept: application/vnd.github+json" --data-binary "@$Bin" $UpUrl
$Uploaded = $Up | ConvertFrom-Json
if ($Uploaded.size -ne $(Get-Item $Bin).Length) {
    Write-Error "ABORT: el asset subido ($($Uploaded.size)) no coincide con el binario local. Revisar."
}
Write-Host "OK: asset $($Uploaded.name) size=$($Uploaded.size) en $Tag  (sha256 $Sha)"
Write-Host "URL: https://github.com/$Owner/$Repo/releases/tag/$Tag"