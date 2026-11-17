param(
    [string]$GameDir = "D:\Program Files (x86)\Steam\steamapps\common\Casualties Unknown",
    [string]$Install
)

$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
$dist = Join-Path $root "dist"
$core = Join-Path $dist "Ferrite\core"
$examples = Join-Path $dist "examples"
$setup = Join-Path $dist "setup"

$cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE ".cargo" }
$rustup = if ($env:RUSTUP_HOME) { $env:RUSTUP_HOME } else { Join-Path $env:USERPROFILE ".rustup" }
$env:CARGO_ENCODED_RUSTFLAGS = @("--remap-path-prefix=$cargoHome=cargo", "--remap-path-prefix=$rustup=rustup", "--remap-path-prefix=$root=ferrite") -join [char]0x1f

if (Test-Path $dist) { Remove-Item $dist -Recurse -Force }
New-Item -ItemType Directory -Force $core, $examples, $setup | Out-Null

cargo build --release --manifest-path (Join-Path $root "Cargo.toml") -p ferrite-loader -p hello-native
if ($LASTEXITCODE -ne 0) { throw "ferrite-loader build failed" }
Copy-Item (Join-Path $root "target\release\version.dll") $dist
Copy-Item (Join-Path $root "target\release\hello_native.dll") $examples

function Build($proj, $out) {
    dotnet build (Join-Path $root $proj) -c Release "-p:GameDir=$GameDir" -o (Join-Path $root $out)
    if ($LASTEXITCODE -ne 0) { throw "$proj build failed" }
}

Build "managed\Ferrite\Ferrite.csproj" "target\managed"
Build "examples\HelloMod\HelloMod.csproj" "target\hello"
Build "examples\HelloPatcher\HelloPatcher.csproj" "target\patcher"
Build "managed\Ferrite.Il2Cpp\Ferrite.Il2Cpp.csproj" "target\il2cpp-managed"
Build "examples\HelloIl2CppCs\HelloIl2CppCs.csproj" "target\hello-il2cpp-cs"
Copy-Item (Join-Path $root "target\managed\Ferrite.dll") $core
Copy-Item (Join-Path $root "libs\*.dll") $core
Copy-Item (Join-Path $root "target\il2cpp-managed\Ferrite.Il2Cpp.dll"), (Join-Path $root "target\il2cpp-managed\Ferrite.Il2Cpp.runtimeconfig.json") $core
Copy-Item (Join-Path $root "target\hello\HelloMod.dll") $examples
Copy-Item (Join-Path $root "target\patcher\HelloPatcher.dll") $examples
Copy-Item (Join-Path $root "target\hello-il2cpp-cs\HelloIl2CppCs.dll") $examples

cargo build --release --manifest-path (Join-Path $root "Cargo.toml") -p ferrite-setup
if ($LASTEXITCODE -ne 0) { throw "ferrite-setup build failed" }
Copy-Item (Join-Path $root "target\release\Ferrite-Setup.exe") $setup

if ($Install) {
    Copy-Item (Join-Path $dist "version.dll"), (Join-Path $dist "Ferrite") $Install -Recurse -Force
    New-Item -ItemType Directory -Force (Join-Path $Install "Ferrite\mods") | Out-Null
    Copy-Item (Join-Path $examples "*.dll") (Join-Path $Install "Ferrite\mods")
    Write-Host "installed to $Install"
}
