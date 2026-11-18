import QtQuick

// Monospace text with the page's em-based letter-spacing (`ls`).
Text {
    property real ls: 0
    property real px: 9
    font.family: "DejaVu Sans Mono"
    font.pixelSize: Math.round(px)
    font.letterSpacing: ls * px
    color: "#b6c0d6"
    renderType: Text.QtRendering
}
