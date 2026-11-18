import QtQuick
import QtQuick.Effects

// The four corner blocks: wordmark, render telemetry, move lanes, live status.
// Read-only views of the snapshot; nothing here writes to the simulation.
Item {
    id: hud
    property var frame
    property var info
    property real pad: 24
    property real strip: 330
    property bool showDesc: true
    property bool showTele: true
    readonly property alias brandBottom: brand.bottomY
    readonly property alias teleBottom: tele.bottomY
    readonly property real timelineTop: lanes.y
    readonly property real statusTop: status.y
    Theme { id: th }

    // ---- brand ----------------------------------------------------------------
    Column {
        id: brand
        readonly property real bottomY: y + height
        x: hud.pad; y: hud.pad; width: hud.strip
        spacing: 0
        Label2 {
            width: parent.width; wrapMode: Text.WordWrap
            text: "QT QUICK 3D · PBR · RUST ENGINE VIA QT BRIDGES"
            px: 8.5; ls: 0.3; color: th.dimmer; lineHeight: 1.25
        }
        Item { width: 1; height: 10 }
        Row {
            spacing: 12
            Item {
                width: title.implicitWidth; height: title.implicitHeight
                Text {
                    id: title
                    text: "RUBIK’S"
                    font.family: th.sans; font.bold: true
                    readonly property int fs: Math.round(Math.max(40, Math.min(66, hud.strip * 0.196)))
                    font.pixelSize: fs
                    font.letterSpacing: -0.035 * fs
                    color: "white"
                    visible: false
                    layer.enabled: true
                }
                Rectangle {
                    id: titleFill
                    anchors.fill: title
                    visible: false
                    layer.enabled: true
                    // The page's vertical text gradient, with a slow sheen sweeping across.
                    gradient: Gradient {
                        GradientStop { position: 0.0; color: "#f6f8fd" }
                        GradientStop { position: 1.0; color: "#9aa6c0" }
                    }
                    Rectangle {
                        id: sheen
                        width: 40; height: parent.height * 2; rotation: 20; y: -parent.height / 2
                        opacity: 0.55
                        gradient: Gradient {
                            orientation: Gradient.Horizontal
                            GradientStop { position: 0.0; color: "#00ffffff" }
                            GradientStop { position: 0.5; color: "#ffffffff" }
                            GradientStop { position: 1.0; color: "#00ffffff" }
                        }
                        SequentialAnimation on x {
                            loops: Animation.Infinite
                            NumberAnimation { from: -80; to: title.width + 80; duration: 1600; easing.type: Easing.InOutQuad }
                            PauseAnimation { duration: 5200 }
                        }
                    }
                }
                MultiEffect {
                    anchors.fill: title
                    source: titleFill
                    maskEnabled: true
                    maskSource: title
                }
            }
            Rectangle {
                anchors.bottom: parent.bottom; anchors.bottomMargin: title.height * 0.16
                width: badge.width + 14; height: badge.height + 5; radius: 2
                color: "transparent"; border.color: th.rule2
                Label2 { id: badge; anchors.centerIn: parent; text: "3×3×3"; px: 9; ls: 0.16 }
            }
        }
        Label2 { text: "C U B E"; px: Math.max(9, title.font.pixelSize * 0.21); ls: 0.62; color: "#8e9bba" }
        Item { width: 1; height: hud.showDesc ? 13 : 0 }
        Rectangle { visible: hud.showDesc; width: Math.min(hud.strip, 216); height: 1; color: th.rule }
        Item { width: 1; height: hud.showDesc ? 12 : 0 }
        Label2 {
            visible: hud.showDesc
            width: Math.min(hud.strip, 222); wrapMode: Text.WordWrap
            text: "Twenty-six rounded cubies, fifty-four bevelled stickers, one key light you can orbit. Physically based plastic, soft shadows and bloom — every pose read from the same Rust engine."
            px: 10; color: th.dim; lineHeight: 1.7
        }
    }

    // ---- telemetry --------------------------------------------------------------
    Column {
        id: tele
        visible: hud.showTele
        readonly property real bottomY: y + height
        anchors.right: parent.right; anchors.rightMargin: hud.pad
        y: hud.pad
        spacing: 0
        Label2 { anchors.right: parent.right; text: "RENDER TELEMETRY"; px: 8; ls: 0.22; color: th.dimmer }
        Item { width: 1; height: 9 }
        Grid {
            anchors.right: parent.right
            columns: 2; columnSpacing: 18; rowSpacing: 4
            Repeater {
                model: hud.frame ? hud.frame.tele.length * 2 : 0
                delegate: Label2 {
                    required property int index
                    readonly property var row: hud.frame.tele[Math.floor(index / 2)]
                    readonly property bool isKey: index % 2 === 0
                    text: isKey ? row[0] : row[1]
                    px: isKey ? 8.5 : 10
                    ls: isKey ? 0.22 : 0
                    color: isKey ? th.dimmer : (row[2] ? th.ok : th.val)
                    horizontalAlignment: isKey ? Text.AlignLeft : Text.AlignRight
                    width: isKey ? implicitWidth : 196
                }
            }
        }
        Item { width: 1; height: 8 }
        Label2 { anchors.right: parent.right; text: "FACE MAP"; px: 8; ls: 0.22; color: th.dimmer }
        Item { width: 1; height: 10 }
        Row {
            anchors.right: parent.right
            spacing: 5
            Repeater {
                model: hud.info ? hud.info.swatches : []
                delegate: Column {
                    required property var modelData
                    spacing: 4
                    Rectangle {
                        width: 17; height: 8; radius: 1
                        color: hud.info.palette[modelData]
                        border.color: Qt.rgba(0, 0, 0, 0.45)
                    }
                    Label2 { anchors.horizontalCenter: parent.horizontalCenter; text: modelData; px: 8; color: th.dimmer }
                }
            }
        }
    }

    // ---- move lanes ---------------------------------------------------------------
    Item {
        id: lanes
        readonly property var tl: hud.frame ? hud.frame.timeline : null
        x: hud.pad
        y: hud.height - hud.pad - height
        width: 69 + 10 * 30.6
        height: 13 + 2 * 22 + 6
        Label2 { x: 69; text: "CYCLE " + (lanes.tl ? String(lanes.tl.cycle).padStart(3, "0") : "001"); px: 8; ls: 0.22; color: th.dimmer }
        Repeater {
            model: 2
            delegate: Item {
                id: lane
                required property int index
                readonly property var moves: lanes.tl ? (index === 0 ? lanes.tl.scramble : lanes.tl.solve) : []
                readonly property int done: lanes.tl ? (index === 0 ? lanes.tl.scrDone : lanes.tl.solDone) : 0
                readonly property int act: lanes.tl ? (index === 0 ? lanes.tl.scrAct : lanes.tl.solAct) : -1
                y: 19 + index * 28
                width: lanes.width; height: 22
                Label2 {
                    anchors.right: parent.left; anchors.rightMargin: -58
                    anchors.verticalCenter: parent.verticalCenter
                    text: lane.index === 0 ? "SCRAMBLE" : "SOLUTION"; px: 8; color: th.dimmer
                }
                Repeater {
                    model: lane.moves.length
                    delegate: Item {
                        required property int index
                        readonly property string modelData: lane.moves[index] || ""
                        readonly property bool isDone: index < lane.done
                        readonly property bool isAct: index === lane.act
                        x: 69 + index * 30.6; width: 26.6; height: 22
                        Rectangle {   // the active chip's halo
                            visible: parent.isAct
                            anchors.centerIn: parent
                            width: 64; height: 64; radius: 32
                            gradient: null
                            color: "transparent"
                            Rectangle {
                                anchors.centerIn: parent; width: 44; height: 44; radius: 22
                                color: Qt.rgba(70/255, 224/255, 140/255, 0.10)
                            }
                        }
                        Rectangle {
                            anchors.fill: parent; radius: 2
                            color: parent.isAct ? Qt.rgba(70/255, 224/255, 140/255, 0.10)
                                 : parent.isDone ? Qt.rgba(158/255, 176/255, 214/255, 0.05) : "transparent"
                            border.color: parent.isAct ? Qt.rgba(70/255, 224/255, 140/255, 0.55)
                                        : parent.isDone ? Qt.rgba(158/255, 176/255, 214/255, 0.2) : th.rule
                            Behavior on color { ColorAnimation { duration: 180 } }
                        }
                        Label2 {
                            anchors.centerIn: parent; text: modelData; px: 9.5
                            color: parent.isAct ? th.ok : parent.isDone ? "#78829a" : th.dimmer
                        }
                        // progress underline on the active chip, driven by the eased k
                        Rectangle {
                            visible: parent.isAct
                            x: 2; y: parent.height - 3; height: 1.5
                            width: (parent.width - 4) * (lanes.tl ? lanes.tl.k : 0)
                            color: th.ok
                        }
                    }
                }
            }
        }
    }

    // ---- status ---------------------------------------------------------------------
    Column {
        id: status
        readonly property var st: hud.frame ? hud.frame.status : null
        readonly property color tone: st ? th.toneColor(st.tone) : th.idle
        anchors.right: parent.right; anchors.rightMargin: hud.pad
        y: hud.height - hud.pad - height
        spacing: 7
        Row {
            anchors.right: parent.right
            spacing: 12
            Item {
                width: 14; height: 14; anchors.verticalCenter: parent.verticalCenter
                Rectangle { anchors.centerIn: parent; width: 14; height: 14; radius: 7; color: "transparent"
                            border.color: th.a(status.tone, 0.18) }
                Rectangle { anchors.centerIn: parent; width: 10; height: 10; radius: 5; color: "transparent"
                            border.color: th.a(status.tone, 0.32) }
                Rectangle {
                    anchors.centerIn: parent; width: 7; height: 7; radius: 3.5; color: status.tone
                    SequentialAnimation on opacity {
                        loops: Animation.Infinite
                        NumberAnimation { to: 0.45; duration: 900; easing.type: Easing.InOutSine }
                        NumberAnimation { to: 1.0; duration: 900; easing.type: Easing.InOutSine }
                    }
                }
            }
            Label2 { text: status.st ? status.st.text : ""; px: 15; ls: 0.17; color: th.ink }
        }
        Label2 { anchors.right: parent.right; text: status.st ? status.st.state : ""; px: 9.5; ls: 0.12; color: th.dim }
        Label2 { anchors.right: parent.right; text: status.st ? status.st.check : ""; px: 9; ls: 0.06
                 color: status.st && status.st.checkPass ? Qt.rgba(70/255, 224/255, 140/255, 0.72) : "#e05a4a" }
    }
}
