$ErrorActionPreference='Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class TodoWindows {
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr handle);
 [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr handle, uint message, IntPtr wparam, IntPtr lparam);
}
'@
$executable=Join-Path $env:LOCALAPPDATA 'LocalTodo/local-todo.exe'
$timer=[Diagnostics.Stopwatch]::StartNew()
$appProcess=Start-Process -FilePath $executable -WindowStyle Hidden -PassThru
$root=$null
for($attempt=0;$attempt -lt 100;$attempt++) {
  $appProcess.Refresh()
  if($appProcess.HasExited){throw 'Installed app exited unexpectedly'}
  if($appProcess.MainWindowHandle -ne 0) {
    $root=[System.Windows.Automation.AutomationElement]::FromHandle($appProcess.MainWindowHandle)
    $ready=$root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty,'新建任务'))
    if($ready){break}
  }
  Start-Sleep -Milliseconds 100
}
if(!$ready){throw 'Installed UI did not become interactive'}
$interactiveMillis=$timer.ElapsedMilliseconds
$windowHandle=$appProcess.MainWindowHandle
[TodoWindows]::PostMessageW($windowHandle,0x0010,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
for($attempt=0;$attempt -lt 50 -and [TodoWindows]::IsWindowVisible($windowHandle);$attempt++){Start-Sleep -Milliseconds 100}
$appProcess.Refresh()
if($appProcess.HasExited -or [TodoWindows]::IsWindowVisible($windowHandle)){throw 'Closing did not retain hidden tray process'}
$second=Start-Process -FilePath $executable -WindowStyle Hidden -PassThru
if(!$second.WaitForExit(10000)){throw 'Second instance did not exit'}
for($attempt=0;$attempt -lt 50 -and ![TodoWindows]::IsWindowVisible($windowHandle);$attempt++){Start-Sleep -Milliseconds 100}
if(![TodoWindows]::IsWindowVisible($windowHandle)){throw 'Second launch did not restore the existing window'}
$root=[System.Windows.Automation.AutomationElement]::FromHandle($windowHandle)
function Click-Button([string]$name) {
  $button=$root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty,$name))
  if(!$button){throw "Button missing: $name"}
  $button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
}
Click-Button '设置与备份'
Start-Sleep -Milliseconds 250
Click-Button '发送测试通知'
$submitted=$null
for($attempt=0;$attempt -lt 50;$attempt++) {
  $submitted=$root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty,'测试通知已提交，请查看 Windows 通知中心。'))
  if($submitted){break};Start-Sleep -Milliseconds 100
}
if(!$submitted){throw 'Installed notification submission was not confirmed'}
Click-Button '关闭设置'
$results=[ordered]@{at=(Get-Date).ToUniversalTime().ToString('o');process_id=$appProcess.Id;interactive_millis=$interactiveMillis;close_retains_tray=$true;single_instance_restores_window=$true;installed_notification_submitted=$true;executable_sha256=(Get-FileHash -LiteralPath $executable).Hash}
$results | ConvertTo-Json | Set-Content -LiteralPath docs/installed-smoke-results.json -Encoding utf8
$results | ConvertTo-Json
