$projectRoot = Split-Path -Parent $PSScriptRoot
if (Test-Path -LiteralPath "$projectRoot/.tools/cargo/bin/cargo.exe") {
  $env:CARGO_HOME = "$projectRoot/.tools/cargo"
  $env:RUSTUP_HOME = "$projectRoot/.tools/rustup"
  $env:PATH = "$projectRoot/.tools/cargo/bin;$env:PATH"
}
