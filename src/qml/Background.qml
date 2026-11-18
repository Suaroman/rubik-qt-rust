import QtQuick

// Backdrop: the page's dark vertical gradient, a soft radial bloom behind the
// cube, the seeded star field (same LCG and keep-out box as the original) and
// the four viewport brackets.
Item {
    id: bg
    property real t: 0
    property real glow: 0           // 0..1, brightens on solve
    property color glowColor: "#2b3a66"
    Theme { id: th }

    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0; color: th.bgTop }
            GradientStop { position: 1; color: th.bgBottom }
        }
    }
    // Radial halo behind the cube.
    Rectangle {
        readonly property real d: Math.min(bg.width, bg.height) * 1.05
        width: d; height: d; radius: d / 2
        anchors.centerIn: parent
        opacity: 0.55 + 0.45 * bg.glow
        gradient: null
        color: "transparent"
        Canvas {
            anchors.fill: parent
            property color c: bg.glowColor
            onCChanged: requestPaint()
            onPaint: {
                const ctx = getContext("2d")
                ctx.reset()
                const r = width / 2
                const g = ctx.createRadialGradient(r, r, 0, r, r, r)
                g.addColorStop(0, Qt.rgba(c.r, c.g, c.b, 0.34))
                g.addColorStop(0.45, Qt.rgba(c.r, c.g, c.b, 0.10))
                g.addColorStop(1, Qt.rgba(c.r, c.g, c.b, 0))
                ctx.fillStyle = g
                ctx.fillRect(0, 0, width, height)
            }
        }
    }

    // Star field: `rnd` is the page's Park-Miller LCG with its seed.
    property var stars: []
    function rebuild() {
        let seed = 20261101
        const rnd = () => { seed = (seed * 48271) % 2147483647; return seed / 2147483647 }
        const w = width, h = height, keep = 0.42 * Math.min(w, h)
        const out = []
        let made = 0, guard = 0
        while (made < 46 && guard < 900) {
            guard++
            const x = rnd() * w, y = rnd() * h
            if (Math.abs(x - w / 2) < keep && Math.abs(y - h / 2) < keep) continue
            const cross = made % 9 === 8
            out.push({ x: x, y: y, cross: cross, delay: cross ? 0 : rnd() * -7 })
            made++
        }
        stars = out
    }
    onWidthChanged: rebuild()
    onHeightChanged: rebuild()
    Component.onCompleted: rebuild()

    Repeater {
        model: bg.stars
        delegate: Item {
            required property var modelData
            x: modelData.x; y: modelData.y
            // `animation: tw 7s ease-in-out infinite alternate`, negative delay.
            readonly property real ph: (((bg.t - modelData.delay) % 14) + 14) % 14 / 7
            readonly property real tri: ph < 1 ? ph : 2 - ph
            readonly property real a: 0.10 + 0.24 * tri * tri * (3 - 2 * tri)
            Rectangle { visible: !modelData.cross; width: 1.4; height: 1.4; color: Qt.rgba(207/255, 224/255, 1, a * 1.4) }
            Label2 {
                visible: modelData.cross
                x: -width / 2; y: -height / 2
                text: "+"; px: 8; color: Qt.rgba(160/255, 182/255, 224/255, 0.17)
            }
        }
    }

    // Viewport brackets.
    Repeater {
        model: 4
        delegate: Item {
            required property int index
            readonly property bool r: index & 1
            readonly property bool b: index & 2
            x: r ? bg.width - 12 - 20 : 12
            y: b ? bg.height - 12 - 20 : 12
            width: 20; height: 20
            Rectangle { x: parent.r ? 19 : 0; width: 1; height: 20; color: th.rule2 }
            Rectangle { y: parent.b ? 19 : 0; width: 20; height: 1; color: th.rule2 }
        }
    }
}
