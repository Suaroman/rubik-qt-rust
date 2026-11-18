# Builds rubik_qt.exe natively on Windows and bundles a portable folder.
#
#   powershell -ExecutionPolicy Bypass -File .\build-windows.ps1          # build + bundle
#   powershell -ExecutionPolicy Bypass -File .\build-windows.ps1 -Run     # ... then launch
#
# Needs: Rust (x86_64-pc-windows-msvc), Visual Studio 2022 Build Tools (C++),
# and Qt >= 6.10 for MSVC 2022 x64 with qtdeclarative, qtquick3d, qtshadertools,
# qtquicktimeline. Pass -QtDir if Qt is not at the default location.
param(
    [string]$QtDir = "C:\Qt\6.12.1\msvc2022_64",
    [switch]$Run
)
# Native tools (cargo) write progress to stderr; judge them by exit code, not by stream.
Set-Location $PSScriptRoot

$qmake = Join-Path $QtDir "bin\qmake.exe"
if (-not (Test-Path $qmake)) { throw "qmake not found at $qmake - pass -QtDir <your Qt msvc2022_64 folder>" }
$env:QMAKE = $qmake                       # Qt Bridge (cxx-qt-build) finds Qt through qmake
$env:PATH = (Join-Path $QtDir "bin") + ";" + $env:PATH

cargo build --release
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

# Portable folder: the exe plus the Qt DLLs, plugins and QML modules it uses.
$dist = Join-Path $PSScriptRoot "dist"
New-Item -ItemType Directory -Force $dist | Out-Null
Copy-Item "target\release\rubik_qt.exe" $dist -Force -ErrorAction SilentlyContinue
if (-not $?) { throw "Could not update dist\rubik_qt.exe - is it still running? Close it and re-run this script." }
& (Join-Path $QtDir "bin\windeployqt.exe") --release --qmldir src\qml --no-translations --no-system-d3d-compiler --no-compiler-runtime (Join-Path $dist "rubik_qt.exe")
if ($LASTEXITCODE -ne 0) { throw "windeployqt failed" }

# App-local MSVC runtime (vcruntime140*.dll, msvcp140*.dll), so the folder also
# runs on PCs without the Visual C++ Redistributable installed.
$crtDirs = @()
foreach ($root in @("${env:ProgramFiles}\Microsoft Visual Studio", "${env:ProgramFiles(x86)}\Microsoft Visual Studio")) {
    $crtDirs += Get-Item "$root\*\*\VC\Redist\MSVC\*\x64\Microsoft.VC*.CRT" -ErrorAction SilentlyContinue
}
$crt = $crtDirs | Sort-Object FullName | Select-Object -Last 1
if ($crt) {
    Copy-Item "$($crt.FullName)\vcruntime140*.dll", "$($crt.FullName)\msvcp140*.dll" $dist -Force
    Write-Host "Bundled MSVC runtime from $($crt.FullName)"
} else {
    Write-Warning "MSVC redistributable DLLs not found; dist\ needs the VC++ Redistributable on other PCs"
}

Write-Host ""
Write-Host "Built: $dist\rubik_qt.exe  (copy the whole dist folder anywhere)" -ForegroundColor Green
if ($Run) { & (Join-Path $dist "rubik_qt.exe") }
