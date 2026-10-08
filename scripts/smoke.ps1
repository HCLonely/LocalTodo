. "$PSScriptRoot/env.ps1"
Set-Location (Split-Path -Parent $PSScriptRoot)
$env:LOCALTODO_TEST_DATA_DIR=Join-Path $PWD ('.tools/smoke-'+(Get-Date -Format 'yyyyMMdd-HHmmss'))
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=9222'
$testProcess=Start-Process -FilePath "$PWD/target/debug/local-todo.exe" -WindowStyle Hidden -PassThru -RedirectStandardOutput "$PWD/.tools/smoke-stdout.log" -RedirectStandardError "$PWD/.tools/smoke-stderr.log"
try { node scripts/desktop-smoke.mjs; $result=$LASTEXITCODE } finally { Stop-Process -Id $testProcess.Id -ErrorAction SilentlyContinue }
exit $result
