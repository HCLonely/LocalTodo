param([int]$ProcessId, [string]$Answer)
$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new()
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class DialogButtons {
 [DllImport("user32.dll")] public static extern IntPtr SendMessageW(IntPtr window, uint message, IntPtr wparam, IntPtr lparam);
}
'@
$condition = [System.Windows.Automation.AndCondition]::new(
  [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ProcessIdProperty,$ProcessId),
  [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty,$Answer))
for ($attempt=0; $attempt -lt 100; $attempt++) {
  $element=[System.Windows.Automation.AutomationElement]::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$condition)
  if ($element) {
    Write-Output "Answering native dialog button: $($element.Current.Name)"
    $buttonHandle=[IntPtr]$element.Current.NativeWindowHandle
    if ($buttonHandle -ne [IntPtr]::Zero) {
      [DialogButtons]::SendMessageW($buttonHandle,0x00F5,[IntPtr]::Zero,[IntPtr]::Zero) | Out-Null
    } else {
      $element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    }
    exit 0
  }
  Start-Sleep -Milliseconds 100
}
throw "Native confirmation button not found: $Answer"
