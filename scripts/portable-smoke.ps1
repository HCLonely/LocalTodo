$ErrorActionPreference='Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PortableWindows {
 [DllImport("user32.dll")] public static extern int GetWindowLongW(IntPtr handle,int index);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr handle);
 [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr handle,uint message,IntPtr wparam,IntPtr lparam);
}
'@
$executable=Join-Path $PWD 'portable/LocalTodo/local-todo.exe'
$directory=Split-Path -Parent $executable
$appProcess=Start-Process -FilePath $executable -ArgumentList '--card' -WorkingDirectory $env:TEMP -WindowStyle Hidden -PassThru
function Find-Element([string]$name) {
  $appProcess.Refresh()
  if($appProcess.MainWindowHandle -eq 0){return $null}
  $root=[System.Windows.Automation.AutomationElement]::FromHandle($appProcess.MainWindowHandle)
  $condition=[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty,$name)
  return $root.FindFirst([System.Windows.Automation.TreeScope]::Subtree,$condition)
}
function Wait-Element([string]$name) {
  for($attempt=0;$attempt -lt 100;$attempt++) {
    $found=Find-Element $name
    if($found){return $found}
    $appProcess.Refresh();if($appProcess.HasExited){throw 'Portable app exited'}
    Start-Sleep -Milliseconds 100
  }
  throw "Element not found: $name"
}
function Click-Element([string]$name) {
  $element=Wait-Element $name
  $pattern=$null
  if($element.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern,[ref]$pattern)){$pattern.Invoke()}
  elseif($element.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern,[ref]$pattern)){$pattern.Toggle()}
  else {throw "Unsupported action: $name"}
}
try {
  $card=Wait-Element '拾序 · 桌面小卡片'
  $pin=Wait-Element '取消置顶'
  $cardHandle=[IntPtr]$card.Current.NativeWindowHandle
  if(([PortableWindows]::GetWindowLongW($cardHandle,-20) -band 8) -eq 0){throw 'Card is not natively topmost'}
  Click-Element '取消置顶'
  Wait-Element '置顶卡片' | Out-Null
  if(([PortableWindows]::GetWindowLongW($cardHandle,-20) -band 8) -ne 0){throw 'Unpin did not change Windows style'}
  if((Get-Content -Raw (Join-Path $directory 'data/card.json')).Trim() -ne 'false'){throw 'Pin preference not persisted under data'}
  Click-Element '置顶卡片'
  Wait-Element '取消置顶' | Out-Null
  Click-Element '隐藏小卡片'
  for($attempt=0;$attempt -lt 50 -and [PortableWindows]::IsWindowVisible($cardHandle);$attempt++){Start-Sleep -Milliseconds 100}
  if([PortableWindows]::IsWindowVisible($cardHandle)){throw 'Card did not hide'}
  $second=Start-Process -FilePath $executable -ArgumentList '--card' -WindowStyle Hidden -PassThru
  if(!$second.WaitForExit(10000)){throw 'Second portable instance did not exit'}
  for($attempt=0;$attempt -lt 50 -and ![PortableWindows]::IsWindowVisible($cardHandle);$attempt++){Start-Sleep -Milliseconds 100}
  if(![PortableWindows]::IsWindowVisible($cardHandle)){throw 'Second --card launch did not reopen card'}
  if(!(Test-Path -LiteralPath (Join-Path $directory 'data/todo.db'))){throw 'Database missing beside executable'}
  if(!(Test-Path -LiteralPath (Join-Path $directory 'data/webview'))){throw 'WebView cache missing beside executable'}
  $webviewPath=Join-Path $directory 'data/webview'
  $profiles=Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | Where-Object {$_.CommandLine -like "*$webviewPath*"}
  if(!$profiles){throw 'WebView2 process did not use portable profile'}
  $results=[ordered]@{at=(Get-Date).ToUniversalTime().ToString('o');portable_process_id=$appProcess.Id;program_directory_data=$true;different_working_directory=$true;webview_profile_in_data=$true;native_pin_toggle=$true;pin_preference_persisted=$true;hidden_card_reopened=$true;single_instance=$true}
  $results | ConvertTo-Json | Set-Content -LiteralPath docs/portable-smoke-results.json -Encoding utf8
  $results | ConvertTo-Json
} finally {
  # Leave the tested portable program and card available for the user.
}
