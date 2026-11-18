#!/usr/bin/env bash
# Installs Qt for Linux x64 (gcc_64) from Qt's official online repository at
# download.qt.io: just the modules this app needs, no Qt account required.
# Distribution packages are usually too old (Ubuntu 24.04 ships Qt 6.4; Qt Bridges
# needs 6.10+).
#
#   tools/install-qt-linux.sh            # Qt 6.12.1 into ~/Qt/6.12.1/gcc_64
#   tools/install-qt-linux.sh 6.12.1     # explicit version
#   QT_ROOT=/opt/Qt tools/install-qt-linux.sh
#
# Needs curl and 7-Zip (Ubuntu/Debian: sudo apt install curl 7zip).
set -euo pipefail

VER="${1:-6.12.1}"
ROOT="${QT_ROOT:-$HOME/Qt}"
TARGET="$ROOT/$VER/gcc_64"
TAG="qt6_${VER//./}"
PKG="${VER//./}"
BASE="https://download.qt.io/online/qtsdkrepository/linux_x64/desktop/$TAG/$TAG"

SEVENZ="$(command -v 7z || command -v 7zz || command -v 7za || true)"
[ -n "$SEVENZ" ] || { echo "7-Zip not found. Install it first:  sudo apt install 7zip" >&2; exit 1; }
command -v curl >/dev/null || { echo "curl not found:  sudo apt install curl" >&2; exit 1; }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TARGET"
echo "Installing Qt $VER (gcc_64) into $TARGET from download.qt.io ..."

# fetch <package dir> <archive-name pattern> <extract into>
fetch() {
    local pkg="$1" pattern="$2" into="$3" index name
    index="$(curl -fsSL "$BASE/$pkg/")" || { echo "Cannot list $BASE/$pkg/ - is Qt $VER published?" >&2; exit 1; }
    name="$(grep -oE "href=\"[^\"]*${pattern}[^\"]*\.7z\"" <<<"$index" | head -1 | sed 's/^href="//; s/"$//')"
    [ -n "$name" ] || { echo "No archive matching '$pattern' in $pkg" >&2; exit 1; }
    echo "  $pattern"
    curl -fsSL -o "$TMP/a.7z" "$BASE/$pkg/$name"
    "$SEVENZ" x -y -o"$into" "$TMP/a.7z" >/dev/null
    rm -f "$TMP/a.7z"
}

MAIN="qt.qt6.$PKG.linux_gcc_64"
for m in qtbase qtdeclarative qtsvg qtwayland; do fetch "$MAIN" "$m-Linux" "$TARGET"; done
fetch "$MAIN" "icu-linux" "$TARGET/lib"          # Qt's own ICU build
for m in qtquick3d qtshadertools qtquicktimeline; do
    fetch "qt.qt6.$PKG.addons.$m.linux_gcc_64" "$m-Linux" "$TARGET"
done

# The archives are relocatable; qt.conf points qmake at its own install prefix.
printf '[Paths]\nPrefix=..\n' > "$TARGET/bin/qt.conf"

echo
echo "Done: $("$TARGET/bin/qmake" -query QT_VERSION) at $TARGET"
echo "Build with:"
echo "  export QMAKE=$TARGET/bin/qmake"
echo "  cargo build --release && ./target/release/rubik_qt"
