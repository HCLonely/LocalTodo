. "$PSScriptRoot/env.ps1"
Set-Location (Split-Path -Parent $PSScriptRoot)
New-Item -ItemType Directory -Path .tools/temp -Force | Out-Null
$env:TEMP = Join-Path $PWD '.tools/temp'
$env:TMP = $env:TEMP
npm run tauri build
