import QtQuick

// A segmented hardware-style meter: dim track, fill, 12-tick overlay.
// `signed` draws a bipolar fill from the centre tick (for vector components).
Item {
    id: m
    property real value: 0          // 0..1, or -1..1 when signed
    property bool signed: false
    property color fill: Qt.rgba(1, 0.73, 0.36, 0.62)
    property color negFill: Qt.rgba(122/255, 134/255, 162/255, 0.42)
    implicitHeight: 6

    readonly property real v: signed ? Math.max(-1, Math.min(1, value)) : Math.max(0, Math.min(1, value))

    Rectangle { anchors.fill: parent; color: Qt.rgba(158/255, 176/255, 214/255, 0.055)
                border.color: Qt.rgba(158/255, 176/255, 214/255, 0.12); border.width: 1 }
    Rectangle {
        y: 0; height: parent.height
        x: m.signed ? (m.v < 0 ? parent.width * (0.5 + m.v / 2) : parent.width / 2) : 0
        width: m.signed ? parent.width * Math.abs(m.v) / 2 : parent.width * m.v
        color: m.signed && m.v < 0 ? m.negFill : m.fill
        Behavior on width { enabled: !m.signed; NumberAnimation { duration: 160; easing.type: Easing.OutCubic } }
    }
    Repeater {
        model: 11
        Rectangle { x: m.width * (index + 1) / 12; width: 1; height: m.height; color: Qt.rgba(10/255, 11/255, 15/255, 0.9) }
    }
    Rectangle { visible: m.signed; x: m.width / 2; y: -1; width: 1; height: m.height + 2
                color: Qt.rgba(158/255, 176/255, 214/255, 0.30) }
}
