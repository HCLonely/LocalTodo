. "$PSScriptRoot/env.ps1"
Set-Location (Split-Path -Parent $PSScriptRoot)
$ErrorActionPreference='Stop'
npm run tauri build -- --no-bundle
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$portableDirectory=Join-Path $PWD 'portable/LocalTodo'
New-Item -ItemType Directory -Path $portableDirectory -Force | Out-Null
Copy-Item -LiteralPath target/release/local-todo.exe -Destination (Join-Path $portableDirectory 'local-todo.exe') -Force
'@start "" "%~dp0local-todo.exe" --card' | Set-Content -LiteralPath (Join-Path $portableDirectory 'Open-Desktop-Card.cmd') -Encoding ascii
'LocalTodo portable 0.2.0. Run local-todo.exe or Open-Desktop-Card.cmd. All app data is in data/ beside the exe. Move the whole folder to keep your tasks. WebView2 Runtime is required.' | Set-Content -LiteralPath (Join-Path $portableDirectory 'README.txt') -Encoding utf8
New-Item -ItemType Directory -Path (Join-Path $portableDirectory 'data') -Force | Out-Null
# Package only a fresh staging directory, never the user's working data/.
$stageRoot=Join-Path $PWD ('.tools/package-'+(Get-Date -Format 'yyyyMMdd-HHmmss'))
$stageDirectory=Join-Path $stageRoot 'LocalTodo'
New-Item -ItemType Directory -Path (Join-Path $stageDirectory 'data') -Force | Out-Null
foreach($name in @('local-todo.exe','Open-Desktop-Card.cmd','README.txt')) {
 Copy-Item -LiteralPath (Join-Path $portableDirectory $name) -Destination $stageDirectory
}
$zip=Join-Path $PWD 'portable/LocalTodo-0.2.0-windows-x64.zip'
Compress-Archive -LiteralPath $stageDirectory -DestinationPath $zip -Force
$checksum=Get-FileHash -LiteralPath $zip -Algorithm SHA256
"$($checksum.Hash.ToLower())  $(Split-Path -Leaf $zip)" | Set-Content -LiteralPath ($zip+'.sha256') -Encoding ascii
Write-Output "Portable executable: $portableDirectory/local-todo.exe"
Write-Output "Portable ZIP: $zip"
