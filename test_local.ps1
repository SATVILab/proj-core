$Url = "file:///C:/path/to/thing"
$localPath = ""
if ($Url -match '^file:///(.+)$') {
    # It turns out if there is a Windows drive letter C:/path, it's correct.
    $localPath = $Matches[1] -replace '/', [System.IO.Path]::DirectorySeparatorChar
}
Write-Host "localPath=$localPath"
