param(
    [string]$Unity = "C:\Program Files\Unity\Hub\Editor\2022.3.62f2\Editor\Unity.exe",
    [string]$Out = "D:\Projects\Ferrite-testgame",
    [switch]$SkipIl2Cpp
)

$ErrorActionPreference = "Stop"
$project = Join-Path $PSScriptRoot "TestGame"
$logs = Join-Path $project "Logs"
New-Item -ItemType Directory -Force $logs | Out-Null

function Build($method, $dir, $name) {
    $log = Join-Path $logs "build-$name.log"
    $argv = @("-batchmode", "-nographics", "-quit", "-projectPath", "`"$project`"", "-buildTarget", "Win64", "-executeMethod", $method, "-out", "`"$dir`"", "-logFile", "`"$log`"")
    $p = Start-Process -FilePath $Unity -ArgumentList $argv -Wait -PassThru -NoNewWindow
    if ($p.ExitCode -ne 0) { throw "$name build failed (exit $($p.ExitCode)), see $log" }
    Write-Host "$name -> $dir"
}

Build "TestGameBuild.BuildMono" (Join-Path $Out "Mono") "Mono"

$il2cpp = Join-Path (Split-Path $Unity) "Data\PlaybackEngines\windowsstandalonesupport\Variations\win64_player_nondevelopment_il2cpp"
if ($SkipIl2Cpp) {
    Write-Host "IL2CPP skipped"
} elseif (Test-Path $il2cpp) {
    Build "TestGameBuild.BuildIl2Cpp" (Join-Path $Out "IL2CPP") "IL2CPP"
} else {
    Write-Host "IL2CPP module not installed, skipped"
}
