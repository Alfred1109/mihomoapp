# Shared preparation logic stages and verifies resources before replacement.
param(
    [string]$MihomoVersion,
    [string]$WinswVersion,
    [string]$WinswSha256
)
$ErrorActionPreference = "Stop"
$arguments = @((Join-Path $PSScriptRoot "prepare-resources.mjs"))
if ($MihomoVersion) { $arguments += @("--mihomo-version", $MihomoVersion) }
if ($WinswVersion) { $arguments += @("--winsw-version", $WinswVersion) }
if ($WinswSha256) { $arguments += @("--winsw-sha256", $WinswSha256) }
& node @arguments
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
