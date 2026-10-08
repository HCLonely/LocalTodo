param(
 [switch]$SkipBuild,
 [string]$OutputDirectory='portable'
)
$ErrorActionPreference='Stop'
$projectRoot=Split-Path -Parent $PSScriptRoot
Set-Location $projectRoot
if (Test-Path -LiteralPath "$projectRoot/.tools/cargo/bin/cargo.exe") {
 $env:CARGO_HOME="$projectRoot/.tools/cargo"
 $env:RUSTUP_HOME="$projectRoot/.tools/rustup"
 $env:PATH="$projectRoot/.tools/cargo/bin;$env:PATH"
}
$version=(Get-Content -LiteralPath src-tauri/tauri.conf.json -Raw | ConvertFrom-Json).version
$packageVersion=(Get-Content -LiteralPath package.json -Raw | ConvertFrom-Json).version
$cargoPackage=Get-Content -LiteralPath src-tauri/Cargo.toml -Raw
$cargoVersion=[regex]::Match($cargoPackage,'(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
if ($version -notmatch '^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$') { throw 'Invalid application version' }
if ($version -ne $packageVersion -or $version -ne $cargoVersion) { throw 'Versions in tauri.conf.json, package.json and src-tauri/Cargo.toml must match' }
if (!$SkipBuild) {
 if (!(Get-Command npm -ErrorAction SilentlyContinue)) { throw 'Node.js and npm are required to build' }
 if (!(Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Rust and Cargo are required to build' }
 if (!(Test-Path -LiteralPath node_modules/.bin/tauri.cmd)) {
  npm ci --prefer-offline --no-audit --no-fund
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
 }
 npm run tauri build -- --no-bundle -- --locked
 if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
$outputRoot=$ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutputDirectory)
$portableDirectory=Join-Path $outputRoot 'LocalTodo'
New-Item -ItemType Directory -Path $portableDirectory -Force | Out-Null
Copy-Item -LiteralPath target/release/local-todo.exe -Destination (Join-Path $portableDirectory 'local-todo.exe') -Force
'@start "" "%~dp0local-todo.exe" --card' | Set-Content -LiteralPath (Join-Path $portableDirectory 'Open-Desktop-Card.cmd') -Encoding ascii
"LocalTodo portable $version. Run local-todo.exe or Open-Desktop-Card.cmd. All app data is in data/ beside the exe. Move the whole folder to keep your tasks. WebView2 Runtime is required." | Set-Content -LiteralPath (Join-Path $portableDirectory 'README.txt') -Encoding utf8
New-Item -ItemType Directory -Path (Join-Path $portableDirectory 'data') -Force | Out-Null
# Package only a fresh staging directory, never the user's working data/.
$stageRoot=Join-Path $PWD ('.tools/package-'+(Get-Date -Format 'yyyyMMdd-HHmmss'))
$stageDirectory=Join-Path $stageRoot 'LocalTodo'
New-Item -ItemType Directory -Path (Join-Path $stageDirectory 'data') -Force | Out-Null
foreach($name in @('local-todo.exe','Open-Desktop-Card.cmd','README.txt')) {
 Copy-Item -LiteralPath (Join-Path $portableDirectory $name) -Destination $stageDirectory
}
$zip=Join-Path $outputRoot "LocalTodo-$version-windows-x64.zip"
Compress-Archive -LiteralPath $stageDirectory -DestinationPath $zip -Force
$checksum=Get-FileHash -LiteralPath $zip -Algorithm SHA256
"$($checksum.Hash.ToLower())  $(Split-Path -Leaf $zip)" | Set-Content -LiteralPath ($zip+'.sha256') -Encoding ascii
Write-Output "Portable executable: $portableDirectory/local-todo.exe"
Write-Output "Portable ZIP: $zip"
