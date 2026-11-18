import QtQuick
import QtQuick.Window
import QtQuick.Effects
import rubik_qt

Window {
    id: win
    readonly property var opts: Rubik.launchOptions()
    readonly property var info: Rubik.staticInfo()
    readonly property var meshes: Rubik.meshes()
    readonly property bool shotMode: opts.shot !== ""
    property var f: Rubik.snapshot()
    property real explode: 0
    property bool helpOpen: false
    property bool hiQuality: opts.quality !== "low"
    property real speed: 1.0
    property int fps: 0

    width: opts.width
    height: opts.height
    minimumWidth: 720
    minimumHeight: 520
    visible: true
    visibility: opts.fullscreen ? Window.FullScreen : Window.Windowed
    color: "#0a0b0f"
    title: "Rubik’s Cube — Qt Quick 3D · " + fps + " fps · H for help · Esc quits"

    FontLoader { source: "fonts/DejaVuSansMono.ttf" }
    FontLoader { source: "fonts/DejaVuSans-Bold.ttf" }
    Theme { id: th }

    // ---- layout: the page's corner blocks around a clear centre strip ----------
    readonly property real pad: Math.max(14, Math.min(28, 0.026 * Math.min(width, height)))
    readonly property real strip: Math.max(250, Math.min(360, 0.23 * width))
    readonly property bool railsVisible: width >= 1060 && height >= 640

    Background {
        id: bg
        anchors.fill: parent
        t: win.f ? win.f.t : 0
        glowColor: win.f && win.f.status.tone === "ok" ? "#1f5a45" : (win.f && win.f.status.tone === "busy" ? "#4a3a1c" : "#26355e")
        Behavior on glowColor { ColorAnimation { duration: 900 } }
    }

    CubeView {
        id: cube
        anchors.fill: parent
        frame: win.f
        info: win.info
        meshes: win.meshes
        explode: win.explode
        hiQuality: win.hiQuality
    }

    Hud {
        id: hud
        visible: !win.opts.only3d
        anchors.fill: parent
        frame: win.f
        info: win.info
        pad: win.pad
        strip: win.strip
        showDesc: win.height >= 860
        showTele: win.width >= 900
    }

    DnaPanel {
        visible: win.railsVisible && !win.opts.only3d
        x: win.pad
        y: hud.brandBottom + 16
        width: win.strip
        height: hud.timelineTop - 22 - y
        frame: win.f
        info: win.info
    }

    LightingPanel {
        visible: win.railsVisible && !win.opts.only3d
        x: win.width - win.pad - win.strip
        y: hud.teleBottom + 16
        width: win.strip
        height: hud.statusTop - 18 - y
        frame: win.f
    }

    // ---- control dock -------------------------------------------------------------
    Rectangle {
        id: dock
        visible: !win.shotMode
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: win.pad
        width: dockRow.width + 16
        height: 34
        radius: 17
        color: Qt.rgba(14/255, 16/255, 23/255, 0.72)
        border.color: Qt.rgba(158/255, 176/255, 214/255, 0.16)
        opacity: dockHover.hovered ? 1 : 0.55
        Behavior on opacity { NumberAnimation { duration: 220 } }
        HoverHandler { id: dockHover }

        component DockButton: Rectangle {
            property string label
            property string key
            property bool on: false
            signal hit()
            width: lbl.width + keyLbl.width + 22; height: 24; radius: 12
            color: on ? Qt.rgba(240/255, 192/255, 112/255, bma.containsMouse ? 0.20 : 0.12)
                      : (bma.containsMouse ? Qt.rgba(158/255, 176/255, 214/255, 0.10) : "transparent")
            border.color: on ? Qt.rgba(240/255, 192/255, 112/255, 0.45) : "transparent"
            Behavior on color { ColorAnimation { duration: 140 } }
            Row {
                anchors.centerIn: parent; spacing: 6
                Label2 { id: lbl; text: label; px: 8.5; ls: 0.16; color: on ? th.amber : th.val }
                Label2 { id: keyLbl; text: key; px: 7.5; color: th.dim }
            }
            MouseArea { id: bma; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: hit() }
        }

        Row {
            id: dockRow
            anchors.centerIn: parent
            spacing: 4
            DockButton { label: win.f && win.f.paused ? "▶ PLAY" : "❚❚ PAUSE"; key: "␣"; on: win.f && win.f.paused; onHit: Rubik.togglePause() }
            DockButton { label: "−"; key: ""; onHit: win.setSpeed(win.speed / 1.5) }
            Label2 { anchors.verticalCenter: parent.verticalCenter; text: win.speed.toFixed(2) + "×"; px: 9; color: th.ink; width: 40; horizontalAlignment: Text.AlignHCenter }
            DockButton { label: "+"; key: ""; onHit: win.setSpeed(win.speed * 1.5) }
            DockButton { label: "EXPLODE"; key: "E"; on: win.explode > 0.5; onHit: win.toggleExplode() }
            DockButton { label: "ORBIT LIGHT"; key: "L"; on: win.f && win.f.light.orbit; onHit: Rubik.toggleOrbit() }
            DockButton { label: "RESET VIEW"; key: "R"; onHit: win.resetView() }
            DockButton { label: win.hiQuality ? "HI-FX" : "LO-FX"; key: "Q"; on: win.hiQuality; onHit: win.hiQuality = !win.hiQuality }
            DockButton { label: "?"; key: "H"; on: win.helpOpen; onHit: win.helpOpen = !win.helpOpen }
        }
    }

    // ---- help overlay ---------------------------------------------------------------
    Rectangle {
        anchors.centerIn: parent
        width: helpCol.width + 48
        height: helpCol.height + 40
        radius: 6
        color: Qt.rgba(9/255, 11/255, 16/255, 0.9)
        border.color: Qt.rgba(158/255, 176/255, 214/255, 0.28)
        opacity: win.helpOpen ? 1 : 0
        visible: opacity > 0
        scale: win.helpOpen ? 1 : 0.96
        Behavior on opacity { NumberAnimation { duration: 180 } }
        Behavior on scale { NumberAnimation { duration: 180; easing.type: Easing.OutCubic } }
        Column {
            id: helpCol
            anchors.centerIn: parent
            spacing: 7
            Label2 { text: "CONTROLS"; px: 9; ls: 0.3; color: th.dimmer }
            Item { width: 1; height: 4 }
            Repeater {
                model: [
                    ["drag", "orbit the cube (with inertia)"],
                    ["double-click", "reset the view"],
                    ["wheel", "zoom"],
                    ["hover cube", "point at any cubie: it glows, a tooltip names it,"],
                    ["", "and its sticker is ringed in the left-hand net"],
                    ["hover net", "or point at a square in the CUBE DNA net (left panel)"],
                    ["space", "pause / resume the clock"],
                    ["+ / −", "time scale (0.05× … 8×)"],
                    ["E", "exploded view"],
                    ["L", "orbit the key light"],
                    ["R", "reset view + zoom"],
                    ["Q", "post-processing quality (MSAA, SSAO, soft shadows)"],
                    ["C", "fire the solve celebration now"],
                    ["F", "fullscreen"],
                    ["H / ?", "this help"],
                    ["Esc", "close help, or quit"],
                ]
                delegate: Row {
                    required property var modelData
                    spacing: 14
                    Label2 { width: 100; text: modelData[0]; px: 10; color: th.amber; horizontalAlignment: Text.AlignRight }
                    Label2 { text: modelData[1]; px: 10; color: th.val }
                }
            }
        }
    }

    // ---- behaviour ------------------------------------------------------------------
    function setSpeed(s) { speed = Math.max(0.05, Math.min(8, s)); Rubik.setSpeed(speed) }
    function toggleExplode() { explodeAnim.to = explode > 0.5 ? 0 : 1; explodeAnim.restart() }
    function resetView() { Rubik.resetView(); cube.zoom = 1 }
    NumberAnimation { id: explodeAnim; target: win; property: "explode"; duration: 900; easing.type: Easing.OutElastic; easing.amplitude: 1.0; easing.period: 0.5 }

    FrameAnimation {
        id: clock
        running: !win.shotMode
        property int frames: 0
        property double t0: Date.now()
        property real tickMs: 0
        property real bindMs: 0
        onTriggered: {
            const a = Date.now()
            const nf = Rubik.tick(frameTime)
            const b = Date.now()
            win.f = nf
            tickMs += b - a
            bindMs += Date.now() - b
            frames++
            const now = Date.now()
            if (now - t0 > 1000) {
                win.fps = Math.round(frames * 1000 / (now - t0)); frames = 0; t0 = now
                if (win.opts.quitAfter > 0) {
                    Rubik.log("[fps] " + win.fps + "  t=" + win.f.t.toFixed(1) + "s  " + win.f.status.text
                              + "  | tick " + (tickMs / Math.max(1, win.fps)).toFixed(2) + " ms, bindings " + (bindMs / Math.max(1, win.fps)).toFixed(2) + " ms")
                    if (win.f.t >= win.opts.quitAfter) Qt.quit()
                }
                tickMs = 0; bindMs = 0
            }
        }
    }

    Item {
        anchors.fill: parent
        focus: true
        Keys.onPressed: (e) => {
            switch (e.key) {
            case Qt.Key_Escape:
                if (win.helpOpen) win.helpOpen = false; else Qt.quit()
                break
            case Qt.Key_Q:
                if (e.modifiers & Qt.ControlModifier) Qt.quit(); else win.hiQuality = !win.hiQuality
                break
            case Qt.Key_L: Rubik.toggleOrbit(); break
            case Qt.Key_Space: Rubik.togglePause(); break
            case Qt.Key_Plus: case Qt.Key_Equal: win.setSpeed(win.speed * 1.5); break
            case Qt.Key_Minus: case Qt.Key_Underscore: win.setSpeed(win.speed / 1.5); break
            case Qt.Key_E: win.toggleExplode(); break
            case Qt.Key_R: win.resetView(); break
            case Qt.Key_C: cube.burst(); break
            case Qt.Key_H: case Qt.Key_Question: win.helpOpen = !win.helpOpen; break
            case Qt.Key_F: win.visibility = win.visibility === Window.FullScreen ? Window.Windowed : Window.FullScreen; break
            default: return
            }
            e.accepted = true
        }
    }

    // ---- headless capture: --shot file.png --at secs ----------------------------------
    Timer { id: burstTimer; interval: 250; onTriggered: cube.burst() }
    Timer {
        id: pointTimer; interval: 400
        onTriggered: { cube.mouse = Qt.point(win.opts.pointX, win.opts.pointY); cube.pickAt(win.opts.pointX, win.opts.pointY); win.f = Rubik.snapshot() }
    }
    Timer {
        id: shotTimer
        interval: win.opts.delay
        onTriggered: {
            win.contentItem.grabToImage(function (r) {
                if (r.saveToFile(win.opts.shot)) Rubik.log("wrote " + win.opts.shot + " at t=" + win.opts.at.toFixed(2) + "s  " + win.width + "x" + win.height)
                else Rubik.log("FAILED to write " + win.opts.shot)
                Qt.quit()
            })
        }
    }
    Component.onCompleted: {
        const scr = Qt.application.screens
        if (opts.quitAfter > 0)
            for (let i = 0; i < scr.length; ++i)
                Rubik.log("[screen " + i + "] " + scr[i].name + "  " + scr[i].width + "x" + scr[i].height
                          + " @" + scr[i].devicePixelRatio + "x  " + Math.round(scr[i].refreshRate) + " Hz")
        if (opts.screen >= 0 && opts.screen < scr.length) {
            const t = scr[opts.screen]
            win.screen = t
            win.x = t.virtualX + Math.max(0, (t.width - win.width) / 2)
            win.y = t.virtualY + Math.max(0, (t.height - win.height) / 2)
        }
        if (shotMode) {
            if (opts.explode) explode = 1
            if (opts.hover >= 0) Rubik.setHover(opts.hover)
            f = Rubik.prestep(opts.at)
            if (opts.celebrate) burstTimer.start()
            if (opts.pointX >= 0) pointTimer.start()
            shotTimer.start()
        }
    }
}
