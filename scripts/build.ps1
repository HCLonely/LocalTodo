. "$PSScriptRoot/env.ps1"
Set-Location (Split-Path -Parent $PSScriptRoot)
New-Item -ItemType Directory -Path .tools/temp -Force | Out-Null
$env:TEMP = Join-Path $PWD '.tools/temp'
$env:TMP = $env:TEMP
npm run tauri build
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Get-ChildItem -LiteralPath target/release/bundle/nsis -Filter '*-setup.exe' | ForEach-Object {
  $checksum=Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256
  "$($checksum.Hash.ToLower())  $($_.Name)" | Set-Content -LiteralPath ($_.FullName+'.sha256') -Encoding ascii
}
