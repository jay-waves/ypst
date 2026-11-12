$ErrorActionPreference = 'Stop'
if (-not [Environment]::Is64BitOperatingSystem -or $env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { throw 'Only Windows x64 is supported.' }
$command = Get-Command ypst.exe -ErrorAction SilentlyContinue
if ($command) { $target = $command.Source; $installDir = Split-Path -Parent $target } else { $installDir = if ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\ypst' }; $target = Join-Path $installDir 'ypst.exe' }
New-Item -ItemType Directory -Force -Path $installDir | Out-Null
$temp = Join-Path $installDir ([IO.Path]::GetRandomFileName())
try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    $cacheBust = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
    $headers = @{ 'Accept' = 'application/vnd.github+json'; 'User-Agent' = 'ypst-installer'; 'Cache-Control' = 'no-cache' }
    $release = Invoke-RestMethod -Headers $headers -Uri "https://api.github.com/repos/jay-waves/ypst/releases/latest?cachebust=$cacheBust"
    $asset = $release.assets | Where-Object { $_.name -eq 'ypst-windows-x64.exe' } | Select-Object -First 1
    if (-not $asset) { throw "Latest release $($release.tag_name) has no ypst-windows-x64.exe asset." }
    Invoke-WebRequest -UseBasicParsing -Headers $headers -Uri "$($asset.browser_download_url)?cachebust=$cacheBust" -OutFile $temp
    Move-Item -Force -LiteralPath $temp -Destination $target
} finally {
    if (Test-Path -LiteralPath $temp) { Remove-Item -LiteralPath $temp -Force }
}
& $target install
if ($LASTEXITCODE -ne 0) { throw 'ypst install failed.' }
Write-Host "Installed ypst to $target"
& $target --version
if (-not ($env:PATH -split ';' | Where-Object { $_ -eq $installDir })) { Write-Host "Add $installDir to PATH: $env:PATH = '$installDir;' + $env:PATH" }
