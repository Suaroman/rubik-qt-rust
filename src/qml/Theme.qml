import QtQuick

// The page's CSS palette, one place. Instantiated per component (it is a
// handful of constants), which keeps every file loadable on its own.
QtObject {
    readonly property color ok: "#46e08c"
    readonly property color ink: "#e4e9f4"
    readonly property color val: "#b6c0d6"
    readonly property color dim: "#5b6479"
    readonly property color dimmer: "#3b4252"
    readonly property color faint: "#3f4657"
    readonly property color rule: Qt.rgba(158/255, 176/255, 214/255, 0.12)
    readonly property color rule2: Qt.rgba(158/255, 176/255, 214/255, 0.26)
    readonly property color amber: "#f0c070"
    readonly property color busy: "#e0b34a"
    readonly property color idle: "#7f8bb0"
    readonly property color bgTop: "#12141c"
    readonly property color bgBottom: "#0a0b0f"
    readonly property string mono: "DejaVu Sans Mono"
    readonly property string sans: "DejaVu Sans"

    function toneColor(t) { return t === "ok" ? ok : (t === "busy" ? busy : idle) }
    function a(c, alpha) { return Qt.rgba(c.r, c.g, c.b, alpha) }
}
