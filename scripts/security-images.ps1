[CmdletBinding()]
param(
    [Parameter(Mandatory)][string[]]$Images,
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '../.local-test/security/images'),
    [switch]$FailOnHigh
)
$ErrorActionPreference = 'Stop'
# Pin the scanner as well as the scanned images; do not publish raw reports automatically.
$scanner = 'aquasec/trivy@sha256:8f6bb947dbc528902e11f2f5be52f7b519f6a39d5d148f41166f2040499768fb'
$reportRoot = [IO.Path]::GetFullPath($OutputDirectory)
$reportPath = Join-Path $reportRoot ([DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss') + '-' + [guid]::NewGuid().ToString('N').Substring(0,8))
New-Item -ItemType Directory -Force -Path $reportPath | Out-Null
$summary = @()
$index = 0
foreach ($image in $Images) {
    if ($image.StartsWith('-') -or $image.Contains("`n") -or $image.Contains("`r")) { throw 'Invalid image reference.' }
    $imageInfo = & docker image inspect $image --format '{{.Id}}'
    if ($LASTEXITCODE -ne 0) { throw "Image must exist locally: $image" }
    $prefix = 'image-{0:D3}' -f $index
    $common = @('run','--rm','-v','/var/run/docker.sock:/var/run/docker.sock','-v',"${reportPath}:/reports",'-v',"${reportRoot}:/scan-cache",$scanner,'--quiet','image','--image-src','docker','--cache-dir','/scan-cache/trivy-cache')
    & docker @common --scanners vuln --format json --output "/reports/$prefix-vulnerabilities.json" $imageInfo
    if ($LASTEXITCODE -ne 0) { throw "Vulnerability scan failed: $image" }
    & docker @common --format cyclonedx --output "/reports/$prefix-sbom.cdx.json" $imageInfo
    if ($LASTEXITCODE -ne 0) { throw "SBOM generation failed: $image" }
    $report = Get-Content -LiteralPath (Join-Path $reportPath "$prefix-vulnerabilities.json") -Raw | ConvertFrom-Json
    $vulnerabilities = @($report.Results | ForEach-Object { $_.Vulnerabilities } | Where-Object { $null -ne $_ })
    $summary += [pscustomobject]@{
        image = $image; image_id = $imageInfo; scanner = $scanner
        checked_at_utc = [DateTime]::UtcNow.ToString('o')
        critical = @($vulnerabilities | Where-Object Severity -eq 'CRITICAL').Count
        high = @($vulnerabilities | Where-Object Severity -eq 'HIGH').Count
        total = $vulnerabilities.Count
        vulnerability_report = "$prefix-vulnerabilities.json"; sbom = "$prefix-sbom.cdx.json"
    }
    $index++
}
ConvertTo-Json -InputObject @($summary) -Depth 5 | Set-Content -LiteralPath (Join-Path $reportPath 'summary.json') -Encoding utf8
$summary | Format-Table image,critical,high,total
Write-Output "Reports: $reportPath"
if ($FailOnHigh -and @($summary | Where-Object { $_.critical -gt 0 -or $_.high -gt 0 }).Count -gt 0) { exit 1 }
