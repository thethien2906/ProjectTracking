$TargetFile = "C:\Code\ProjectTracking\target\debug\project_tracker.exe"
$ShortcutFile = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Project Tracker.lnk"

if (Test-Path $TargetFile) {
    $WshShell = New-Object -ComObject WScript.Shell
    $Shortcut = $WshShell.CreateShortcut($ShortcutFile)
    $Shortcut.TargetPath = $TargetFile
    $Shortcut.Description = "Manage your projects with ease."
    # The icon will be automatically pulled from the embedded resource we set up earlier
    $Shortcut.IconLocation = $TargetFile
    $Shortcut.Save()
    Write-Host "Success: Project Tracker is now searchable in your Start Menu!" -ForegroundColor Cyan
} else {
    Write-Error "Could not find project_tracker.exe. Please run 'cargo run' first to build it."
}
