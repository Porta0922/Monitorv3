# Pure release selection shared with the updater and its regression tests.
function Select-MonitorRelease($Releases,[string]$Channel) {
    if ($Channel -notin @('candidate','stable')) {throw 'Invalid update channel.'}
    $matches=@($Releases | Where-Object {
        !$_.draft -and $_.tag_name -cmatch '^agent-v[0-9]+\.[0-9]+\.[0-9]+$' -and
        ($Channel -eq 'candidate' -or !$_.prerelease)
    } | Sort-Object {[version]$_.tag_name.Substring(7)} -Descending)
    if ($matches.Count) {return $matches[0]}
    return $null
}
function Get-MonitorWindowsAsset($Release) {
    if ($Release.tag_name -cnotmatch '^agent-v[0-9]+\.[0-9]+\.[0-9]+$') {throw 'Invalid release tag.'}
    $version=$Release.tag_name.Substring(7)
    $name="monitor-agent-$version-windows-x64.msi"
    $assets=@($Release.assets|Where-Object {$_.name -ceq $name -and $_.state -eq 'uploaded'})
    if ($assets.Count -ne 1 -or $assets[0].digest -cnotmatch '^sha256:[a-f0-9]{64}$') {throw 'Release package or GitHub digest missing.'}
    $expectedUrl="https://github.com/Porta0922/Monitorv3/releases/download/agent-v$version/$name"
    if ($assets[0].browser_download_url -cne $expectedUrl) {throw 'Release asset origin mismatch.'}
    return $assets[0]
}
