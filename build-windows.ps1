# Builds rubik_qt.exe natively on Windows and bundles a portable dist\ folder.
#
#   .\build-windows.ps1                 build + bundle, using the Qt it finds
#   .\build-windows.ps1 -InstallQt      first download Qt from download.qt.io if none is found
#   .\build-windows.ps1 -Run            ... then launch the app
#   .\build-windows.ps1 -QtDir D:\Qt\6.12.1\msvc2022_64     use a specific Qt
#
# (If scripts are blocked: powershell -ExecutionPolicy Bypass -File .\build-windows.ps1 ...)
#
# Needs: Rust (x86_64-pc-windows-msvc), Visual Studio 2022 Build Tools with the C++
# workload, and Qt >= 6.10 for MSVC 2022 x64 with qtdeclarative, qtquick3d,
# qtshadertools and qtquicktimeline. -InstallQt fetches exactly those.
param(
    [string]$QtDir = "",
    [switch]$InstallQt,
    [string]$QtVersion = "6.12.1",
    [string]$QtRoot = "C:\Qt",
    [switch]$Run
)
# Native tools (cargo) write progress to stderr; judge them by exit code, not by stream.
Set-Location $PSScriptRoot
$ProgressPreference = "SilentlyContinue"   # Invoke-WebRequest is many times slower with a progress bar

function Fail([string]$msg) { Write-Host ""; Write-Host $msg -ForegroundColor Red; exit 1 }

# ---- prerequisites --------------------------------------------------------------
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $userCargo = Join-Path $env:USERPROFILE ".cargo\bin"
    if (Test-Path (Join-Path $userCargo "cargo.exe")) { $env:PATH = "$userCargo;$env:PATH" }
    else { Fail "Rust is not installed. Install it from https://rustup.rs (default MSVC toolchain), open a new terminal, and re-run." }
}
$linkers = @()
foreach ($root in @("${env:ProgramFiles}\Microsoft Visual Studio", "${env:ProgramFiles(x86)}\Microsoft Visual Studio")) {
    $linkers += Get-Item "$root\*\*\VC\Tools\MSVC\*\bin\Hostx64\x64\link.exe" -ErrorAction SilentlyContinue
}
if (-not $linkers) {
    Fail ("The MSVC C++ build tools are missing. Install them with:`n" +
          "  winget install Microsoft.VisualStudio.2022.BuildTools --override `"--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended`"`n" +
          "or the Visual Studio Installer ('Desktop development with C++'), then re-run.")
}

# ---- find Qt ------------------------------------------------------------------------
function Get-QtVersion([string]$dir) {
    $q = Join-Path $dir "bin\qmake.exe"
    if (-not (Test-Path $q)) { return $null }
    $v = & $q -query QT_VERSION 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $v) { return $null }
    return [version]$v.Trim()
}

function Find-Qt {
    if ($QtDir) { return $QtDir }
    if ($env:QMAKE -and (Test-Path $env:QMAKE)) { return (Split-Path (Split-Path $env:QMAKE)) }
    $found = Get-ChildItem "$QtRoot\6.*\msvc20*_64" -Directory -ErrorAction SilentlyContinue |
        ForEach-Object { [pscustomobject]@{ Dir = $_.FullName; Ver = (Get-QtVersion $_.FullName) } } |
        Where-Object { $_.Ver -and $_.Ver -ge [version]"6.10" } |
        Sort-Object Ver -Descending | Select-Object -First 1
    if ($found) { return $found.Dir }
    return $null
}

# Qt's official online repository, unpacked with Windows' built-in tar.exe (libarchive
# reads .7z). No Qt account needed: these are the same archives the Qt installer uses.
function Install-Qt {
    $target = Join-Path $QtRoot "$QtVersion\msvc2022_64"
    $tag = "qt6_" + ($QtVersion -replace "\.", "")
    $base = "https://download.qt.io/online/qtsdkrepository/windows_x86/desktop/$tag/${tag}_msvc2022_64"
    $pkgVer = $QtVersion -replace "\.", ""
    $wanted = @(
        @{ Pkg = "qt.qt6.$pkgVer.win64_msvc2022_64"; Mods = @("qtbase", "qtdeclarative", "qtsvg", "qttools") },
        @{ Pkg = "qt.qt6.$pkgVer.addons.qtquick3d.win64_msvc2022_64"; Mods = @("qtquick3d") },
        @{ Pkg = "qt.qt6.$pkgVer.addons.qtshadertools.win64_msvc2022_64"; Mods = @("qtshadertools") },
        @{ Pkg = "qt.qt6.$pkgVer.addons.qtquicktimeline.win64_msvc2022_64"; Mods = @("qtquicktimeline") }
    )
    Write-Host "Installing Qt $QtVersion (MSVC 2022 x64) into $target from download.qt.io ..." -ForegroundColor Cyan
    $tmp = Join-Path $env:TEMP "qt-download-$QtVersion"
    New-Item -ItemType Directory -Force $tmp, $target | Out-Null
    foreach ($w in $wanted) {
        try { $index = (Invoke-WebRequest -UseBasicParsing "$base/$($w.Pkg)/").Content }
        catch { Fail "Could not list $base/$($w.Pkg)/ - is Qt $QtVersion published for MSVC 2022? ($($_.Exception.Message))" }
        foreach ($m in $w.Mods) {
            $name = [regex]::Matches($index, 'href="([^"]*' + $m + '-Windows[^"]*\.7z)"') | Select-Object -First 1 |
                ForEach-Object { $_.Groups[1].Value }
            if (-not $name) { Fail "Module $m not found in $($w.Pkg)" }
            $file = Join-Path $tmp $name
            if (-not (Test-Path $file)) {
                Write-Host "  downloading $m ..."
                Invoke-WebRequest -UseBasicParsing "$base/$($w.Pkg)/$name" -OutFile $file
            }
            Write-Host "  extracting  $m ..."
            tar.exe -xf $file -C $target
            if ($LASTEXITCODE -ne 0) { Fail "Extracting $name failed" }
        }
    }
    # The archives are relocatable; qt.conf points qmake at its own install prefix.
    Set-Content -Path (Join-Path $target "bin\qt.conf") -Value "[Paths]`r`nPrefix=..`r`n" -Encoding ASCII
    Remove-Item -Recurse -Force $tmp
    return $target
}

$QtDir = Find-Qt
if (-not $QtDir) {
    if ($InstallQt) { $QtDir = Install-Qt }
    else {
        Fail ("No Qt >= 6.10 for MSVC 2022 x64 found (looked in $QtRoot\6.*\msvc20*_64 and `$env:QMAKE).`n" +
              "Either let this script fetch it (about 300 MB download, no Qt account needed):`n" +
              "  .\build-windows.ps1 -InstallQt`n" +
              "or point it at an existing install:`n" +
              "  .\build-windows.ps1 -QtDir <path to Qt>\6.x.y\msvc2022_64")
    }
}
$qmake = Join-Path $QtDir "bin\qmake.exe"
$ver = Get-QtVersion $QtDir
if (-not $ver) { Fail "qmake not found or not working at $qmake" }
if ($ver -lt [version]"6.10") { Fail "Qt $ver at $QtDir is too old: Qt Bridges needs Qt 6.10 or newer (try -InstallQt)." }
foreach ($need in @("qml\QtQuick3D", "qml\QtQuick\Timeline", "bin\windeployqt.exe")) {
    if (-not (Test-Path (Join-Path $QtDir $need))) {
        Fail "Qt at $QtDir is missing $need. Add the Qt Quick 3D, Qt Quick Timeline and Qt Shader Tools modules (or use -InstallQt)."
    }
}
Write-Host "Using Qt $ver at $QtDir" -ForegroundColor Cyan
$env:QMAKE = $qmake                       # Qt Bridge (cxx-qt-build) finds Qt through qmake
$env:PATH = (Join-Path $QtDir "bin") + ";" + $env:PATH

# ---- build ----------------------------------------------------------------------------
cargo build --release
if ($LASTEXITCODE -ne 0) { Fail "cargo build failed (see the errors above)" }

# Portable folder: the exe plus the Qt DLLs, plugins and QML modules it uses.
$dist = Join-Path $PSScriptRoot "dist"
New-Item -ItemType Directory -Force $dist | Out-Null
Copy-Item "target\release\rubik_qt.exe" $dist -Force -ErrorAction SilentlyContinue
if (-not $?) { Fail "Could not update dist\rubik_qt.exe - is it still running? Close it and re-run this script." }
& (Join-Path $QtDir "bin\windeployqt.exe") --release --qmldir src\qml --no-translations --no-system-d3d-compiler --no-compiler-runtime (Join-Path $dist "rubik_qt.exe")
if ($LASTEXITCODE -ne 0) { Fail "windeployqt failed" }

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
