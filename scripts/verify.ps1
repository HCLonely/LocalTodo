. "$PSScriptRoot/env.ps1"
Set-Location (Split-Path -Parent $PSScriptRoot)
foreach ($step in @('cargo fmt --all --check','cargo clippy --workspace --all-targets -- -D warnings','cargo test --workspace','npm test','npm run build')) {
  Write-Host "Running $step"
  Invoke-Expression $step
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
