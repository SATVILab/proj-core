$Url = "file:///C:/Users/RUNNER~1/AppData/Local/Temp/fake-release/proj_windows_amd64.exe"
if ($Url -match '^file:///(.+)$') {
    # Convert forward slashes to OS path separators and handle Windows drive
    # letters (e.g. file:///C:/path -> C:\path).
    $localPath = $Matches[1] -replace '/', [System.IO.Path]::DirectorySeparatorChar
    Write-Host "Local path is: $localPath"
}
