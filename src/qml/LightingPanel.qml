import QtQuick
import rubik_qt

// Right rail: a world-space instrument, not a second renderer. Every number is
// read from the engine's own light vector, spin and Lambert term, so the panel
// cannot disagree with the cube. The diagram keeps the main view's conventions:
// fixed camera on +z, `f / (camZ - z)` projection at its own focal length.
Item {
    id: panel
    property var frame
    readonly property var lt: frame ? frame.light : null
    readonly property real gap: Math.max(8, Math.min(14, 0.014 * Math.min(parent ? parent.width : 1440, parent ? parent.height : 900)))
    signal orbitToggled()
    Theme { id: th }

    Caption { id: cap; width: parent.width; leftText: "LIGHTING"; rightText: "SPATIAL ANALYSIS"; note: "WORLD SPACE · +Z TO CAM" }

    Canvas {
        id: dia
        anchors.top: cap.bottom
        anchors.bottom: readout.top; anchors.bottomMargin: panel.gap
        width: parent.width
        renderStrategy: Canvas.Cooperative
        antialiasing: true

        readonly property real camZ: 8.6       // diagram camera distance
        readonly property real cubeH: 0.80     // reference cube half-extent
        readonly property real orbitR: 2.30    // the light is drawn at dir x r
        readonly property real axisLen: 1.18
        readonly property real gridY: -1.42
        readonly property var faceCorners: [[1, 3, 7, 5], [0, 2, 6, 4], [2, 3, 7, 6], [0, 1, 5, 4], [4, 5, 7, 6], [0, 1, 3, 2]]
        readonly property var edges: [[0, 1], [2, 3], [4, 5], [6, 7], [0, 2], [1, 3], [4, 6], [5, 7], [0, 4], [1, 5], [2, 6], [3, 7]]
        readonly property var dirs: [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]]
        readonly property var axisNames: ["+X", "-X", "+Y", "-Y", "+Z", "-Z"]

        property real vf: 1; property real vox: 0; property real voy: 0
        function pj(p) { const s = vf / (camZ - p[2]); return [vox + p[0] * s, voy - p[1] * s] }
        function ap(m, v) { return [m[0] * v[0] + m[1] * v[1] + m[2] * v[2], m[3] * v[0] + m[4] * v[1] + m[5] * v[2], m[6] * v[0] + m[7] * v[1] + m[8] * v[2]] }
        function dot(a, b) { return a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
        function line(ctx, a, b, col, w) { ctx.beginPath(); ctx.moveTo(a[0], a[1]); ctx.lineTo(b[0], b[1]); ctx.strokeStyle = col; ctx.lineWidth = w || 1; ctx.stroke() }
        function arrow(ctx, x, y, dx, dy, len, col) {
            const l = Math.max(1e-6, Math.hypot(dx, dy)); dx /= l; dy /= l
            const px = -dy, py = dx, w = len * 0.42
            ctx.beginPath(); ctx.moveTo(x - dx * len + px * w, y - dy * len + py * w); ctx.lineTo(x, y)
            ctx.lineTo(x - dx * len - px * w, y - dy * len - py * w); ctx.strokeStyle = col; ctx.lineWidth = 1; ctx.stroke()
        }

        onPaint: {
            const ctx = getContext("2d")
            ctx.reset()
            const f = panel.frame
            if (!f) return
            const L = f.light, M = f.spinM, ld = L.dir
            const dw = width, dh = height
            vf = 1.18 * Math.min(dw, dh * 1.1); vox = dw / 2; voy = dh * 0.53
            const cx = x => Math.max(20, Math.min(dw - 20, x)), cy = y => Math.max(8, Math.min(dh - 7, y))
            const lambert = n => Math.max(0, dot(n, ld))

            // reference grid below everything the cube can sweep
            const ext = 1.5
            for (let i = 0; i <= 5; ++i) {
                const t = -ext + 2 * ext * i / 5
                line(ctx, pj([t, gridY, -ext]), pj([t, gridY, ext]), "rgba(158,176,214,0.075)")
                line(ctx, pj([-ext, gridY, t]), pj([ext, gridY, t]), "rgba(158,176,214,0.075)")
            }
            // world axes
            const o = pj([0, 0, 0])
            for (let i = 0; i < 6; ++i) {
                const a = dirs[i], e = pj([a[0] * axisLen, a[1] * axisLen, a[2] * axisLen])
                line(ctx, o, e, i % 2 === 0 ? "rgba(170,190,228,0.46)" : "rgba(170,190,228,0.17)")
                if (i % 2 === 0) arrow(ctx, e[0], e[1], e[0] - o[0], e[1] - o[1], 6, "rgba(170,190,228,0.52)")
            }
            ctx.beginPath(); ctx.arc(o[0], o[1], 2.6, 0, Math.PI * 2); ctx.strokeStyle = "rgba(206,220,246,0.55)"; ctx.stroke()
            ctx.beginPath(); ctx.arc(o[0], o[1], 0.9, 0, Math.PI * 2); ctx.fillStyle = "rgba(206,220,246,0.75)"; ctx.fill()

            // reference cube in the cube's real orientation, plus depth-sorted guides
            const h = cubeH
            const v = []
            for (let i = 0; i < 8; ++i) v.push(ap(M, [i & 1 ? h : -h, i & 2 ? h : -h, i & 4 ? h : -h]))
            const items = []
            const front = []
            for (let i = 0; i < 6; ++i) {
                const cs = faceCorners[i], n = ap(M, dirs[i])
                const c = [0, 0, 0]
                for (const k of cs) for (let a = 0; a < 3; ++a) c[a] += v[k][a] / 4
                const facing = dot(n, [-c[0], -c[1], camZ - c[2]]) > 0
                items.push({ d: c[2], kind: "face", i: i, facing: facing, lit: lambert(n) })
                if (facing) front.push(i)
            }
            for (let i = 0; i < 12; ++i) items.push({ d: (v[edges[i][0]][2] + v[edges[i][1]][2]) / 2, kind: "edge", i: i })
            const se = L.elev, ce = L.elevCos
            for (let i = 0; i < 36; ++i) {
                const a0 = L.az0 + i / 36 * Math.PI * 2, a1 = L.az0 + (i + 1) / 36 * Math.PI * 2
                const p0 = [ce * Math.sin(a0) * orbitR, Math.sin(se) * orbitR, ce * Math.cos(a0) * orbitR]
                const p1 = [ce * Math.sin(a1) * orbitR, Math.sin(se) * orbitR, ce * Math.cos(a1) * orbitR]
                items.push({ d: (p0[2] + p1[2]) / 2, kind: "ring", a: pj(p0), b: pj(p1) })
            }
            let perp = [-ld[2], 0, ld[0]]   // dir x (0,1,0)
            const pl = Math.hypot(perp[0], perp[2])
            perp = pl > 1e-6 ? [perp[0] / pl, 0, perp[2] / pl] : [1, 0, 0]
            for (let i = -1; i <= 1; ++i) {
                const off = perp.map(x => x * i * 0.62)
                const from = [ld[0] * 1.8 + off[0], ld[1] * 1.8 + off[1], ld[2] * 1.8 + off[2]]
                const to = [ld[0] * 0.95 + off[0] * 0.62, ld[1] * 0.95 + off[1] * 0.62, ld[2] * 0.95 + off[2] * 0.62]
                items.push({ d: (from[2] + to[2]) / 2, kind: "ray", a: pj(from), b: pj(to) })
            }
            const pos = ld.map(x => x * orbitR), beamTo = ld.map(x => x * 0.98)
            items.push({ d: pos[2] * 0.62, kind: "beam", a: pj(pos), b: pj(beamTo) })
            items.push({ d: pos[2], kind: "light", a: pj(pos) })
            items.sort((a, b) => a.d - b.d)

            for (const it of items) {
                if (it.kind === "face") {
                    const q = faceCorners[it.i].map(k => pj(v[k]))
                    ctx.beginPath(); ctx.moveTo(q[0][0], q[0][1]); for (let k = 1; k < 4; ++k) ctx.lineTo(q[k][0], q[k][1]); ctx.closePath()
                    ctx.fillStyle = "rgba(146,170,214," + (it.facing ? 0.07 : 0.025) + ")"; ctx.fill()
                    if (it.lit > 0.002) { ctx.fillStyle = "rgba(255,190,110," + (0.17 * it.lit) + ")"; ctx.fill() }
                    if (it.facing) { ctx.strokeStyle = "rgba(186,204,238,0.50)"; ctx.lineWidth = 1; ctx.stroke() }
                } else if (it.kind === "edge") {
                    const t01 = Math.max(0, Math.min(1, (it.d / (h * 1.74) + 1) / 2))
                    line(ctx, pj(v[edges[it.i][0]]), pj(v[edges[it.i][1]]), "rgba(176,196,232," + (0.09 + 0.34 * t01 * t01) + ")")
                } else if (it.kind === "ring") {
                    const rt = Math.max(0, Math.min(1, (it.d / orbitR + 1) / 2))
                    line(ctx, it.a, it.b, "rgba(255,186,92," + ((L.orbit ? 0.10 : 0.05) + 0.17 * rt) + ")")
                } else if (it.kind === "ray") {
                    ctx.setLineDash([2, 4]); line(ctx, it.a, it.b, "rgba(255,205,140,0.20)"); ctx.setLineDash([])
                    arrow(ctx, it.b[0], it.b[1], it.b[0] - it.a[0], it.b[1] - it.a[1], 5, "rgba(255,205,140,0.30)")
                } else if (it.kind === "beam") {
                    line(ctx, it.a, it.b, "rgba(255,214,150,0.42)")
                    arrow(ctx, it.b[0], it.b[1], it.b[0] - it.a[0], it.b[1] - it.a[1], 7, "rgba(255,214,150,0.60)")
                } else {
                    const p = it.a
                    const g = ctx.createRadialGradient(p[0], p[1], 0, p[0], p[1], 16)
                    g.addColorStop(0, "rgba(255,206,120,0.40)"); g.addColorStop(0.45, "rgba(255,180,80,0.11)"); g.addColorStop(1, "rgba(255,170,60,0)")
                    ctx.fillStyle = g; ctx.fillRect(p[0] - 16, p[1] - 16, 32, 32)
                    ctx.beginPath(); ctx.arc(p[0], p[1], 4.8, 0, Math.PI * 2); ctx.strokeStyle = "rgba(255,214,150,0.40)"; ctx.lineWidth = 1; ctx.stroke()
                    ctx.beginPath(); ctx.arc(p[0], p[1], 2.1, 0, Math.PI * 2); ctx.fillStyle = "#ffe2ae"; ctx.fill()
                }
            }

            // 3x3 construction lines on camera-facing faces
            for (const fi of front) {
                const n0 = dirs[fi]
                const refv = Math.abs(n0[1]) > 0.9 ? [1, 0, 0] : [0, 1, 0]
                // face_basis: u = normalize(ref x n), v = n x u
                const u0 = [refv[1] * n0[2] - refv[2] * n0[1], refv[2] * n0[0] - refv[0] * n0[2], refv[0] * n0[1] - refv[1] * n0[0]]
                const v0 = [n0[1] * u0[2] - n0[2] * u0[1], n0[2] * u0[0] - n0[0] * u0[2], n0[0] * u0[1] - n0[1] * u0[0]]
                const uu = ap(M, u0), vv = ap(M, v0), nn = ap(M, n0)
                const cc = nn.map(x => x * h)
                const mk = q => pj([0, 1, 2].map(a => cc[a] + uu[a] * q[0] + vv[a] * q[1]))
                for (const kk of [-1, 1]) {
                    const g3 = h * kk / 3
                    line(ctx, mk([-h, g3]), mk([h, g3]), "rgba(186,204,238,0.13)")
                    line(ctx, mk([g3, -h]), mk([g3, h]), "rgba(186,204,238,0.13)")
                }
            }

            // labels last: never occluded in an instrument
            ctx.font = "7.6px 'DejaVu Sans Mono'"; ctx.textAlign = "center"; ctx.textBaseline = "middle"
            for (let i = 0; i < 4; ++i) {
                const a = dirs[i], pe = pj([a[0] * axisLen * 1.16, a[1] * axisLen * 1.16, a[2] * axisLen * 1.16])
                const dx = pe[0] - o[0], dy = pe[1] - o[1], dl = Math.max(1, Math.hypot(dx, dy))
                ctx.fillStyle = i % 2 === 0 ? "rgba(198,212,240,0.72)" : "rgba(150,166,198,0.32)"
                ctx.fillText(axisNames[i], cx(pe[0] + dx / dl * 8), cy(pe[1] + dy / dl * 8))
            }
            ctx.fillStyle = "rgba(198,212,240,0.62)"; ctx.fillText("+Z · cam", cx(o[0]), cy(o[1] + 15))
            ctx.fillStyle = "rgba(150,166,198,0.30)"; ctx.fillText("−Z", cx(o[0]), cy(o[1] - 15))
            const lp = pj(pos), lx = lp[0] < vox ? -1 : 1
            ctx.textAlign = lx < 0 ? "right" : "left"
            ctx.fillStyle = "rgba(255,205,130,0.82)"; ctx.fillText("light", cx(lp[0] + lx * 15), cy(lp[1] - 12))
        }
    }

    // ---- readout ---------------------------------------------------------------
    Column {
        id: readout
        anchors.bottom: parent.bottom
        width: parent.width
        spacing: 0
        SectionHead { width: parent.width; leftText: "LIGHT VECTOR"; rightText: "toward source · |l|=1" }
        Item { width: 1; height: 3 }
        Repeater {
            model: 3
            delegate: Item {
                required property int index
                width: readout.width; height: 14
                Label2 { anchors.verticalCenter: parent.verticalCenter; text: ["X", "Y", "Z"][index]; px: 8.5; color: "#79839c" }
                Meter { x: 26; anchors.verticalCenter: parent.verticalCenter; width: parent.width - 26 - 62; height: 6
                        signed: true; value: panel.lt ? panel.lt.dir[index] : 0; fill: Qt.rgba(150/255, 178/255, 225/255, 0.58) }
                Label2 { anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter; text: panel.lt ? panel.lt.dirText[index] : ""; px: 9 }
            }
        }
        Item { width: 1; height: panel.gap }
        SectionHead { width: parent.width; leftText: "FACE ILLUMINATION"; rightText: "n = spin · n̂" }
        Item { width: 1; height: 3 }
        Repeater {
            model: 6
            delegate: Item {
                required property int index
                readonly property var modelData: panel.lt ? panel.lt.faces[index] : ({ label: "", color: "black", lit: 0, text: "", raw: 0 })
                width: readout.width; height: 14
                Label2 { anchors.verticalCenter: parent.verticalCenter; text: modelData.label; px: 8.5; color: "#79839c" }
                Rectangle { x: 22; anchors.verticalCenter: parent.verticalCenter; width: 13; height: 6; radius: 1; color: modelData.color }
                Meter { x: 42; anchors.verticalCenter: parent.verticalCenter; width: parent.width - 42 - 80; height: 6
                        value: modelData.lit; fill: Qt.rgba(1, 186/255, 92/255, 0.62) }
                Label2 { anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter; text: modelData.text; px: 9
                         color: modelData.raw <= 0 ? "#59627a" : th.val }
            }
        }
        Item { width: 1; height: panel.gap }
        Rectangle { width: parent.width; height: 1; color: th.rule }
        Item { width: 1; height: 11 }
        Repeater {
            model: 6
            delegate: Item {
                required property int index
                readonly property var modelData: panel.lt ? panel.lt.meta[index] : ["", "", false]
                width: readout.width; height: 14
                Label2 { anchors.verticalCenter: parent.verticalCenter; text: modelData[0]; px: 7.5; ls: 0.18; color: th.dimmer }
                Label2 { anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter; text: modelData[1]; px: 8.5
                         color: modelData[2] && panel.lt.orbit ? th.amber : "#8b96b0" }
            }
        }
        Item { width: 1; height: panel.gap }
        // The one control on the panel.
        Rectangle {
            id: btn
            readonly property bool on: panel.lt ? panel.lt.orbit : false
            width: btnRow.width + 16; height: 22; radius: 2
            color: on ? Qt.rgba(240/255, 192/255, 112/255, ma.containsMouse ? 0.16 : 0.08)
                      : (ma.containsMouse ? Qt.rgba(158/255, 176/255, 214/255, 0.06) : "transparent")
            border.color: on ? Qt.rgba(240/255, 192/255, 112/255, 0.5) : Qt.rgba(158/255, 176/255, 214/255, 0.26)
            Behavior on color { ColorAnimation { duration: 150 } }
            Row {
                id: btnRow
                anchors.centerIn: parent; spacing: 7
                Label2 { text: "ORBIT LIGHT"; px: 8; ls: 0.18; color: btn.on ? th.amber : th.dim }
                Label2 { text: "L"; px: 7; ls: 0.18; color: btn.on ? th.amber : th.dim; opacity: 0.55 }
            }
            MouseArea { id: ma; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                        onClicked: Rubik.toggleOrbit() }
        }
    }

    Connections {
        target: panel
        function onFrameChanged() { dia.requestPaint() }
    }
}
