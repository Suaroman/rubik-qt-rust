import QtQuick
import rubik_qt

// Left rail: the cube's 54 stickers as an unfolded net, read straight from the
// engine every frame. While a quarter turn is in flight, every sticker it moves
// travels along its permutation arc, driven by the same eased k that turns the
// 3D slice — so diagram and plastic stay in lockstep.
Item {
    id: panel
    property var frame
    property var info
    readonly property var dna: frame ? frame.dna : null
    Theme { id: th }

    Caption {
        id: cap
        width: parent.width
        leftText: "CUBE DNA"; rightText: "STATE OBSERVATORY"; note: "CANONICAL NET · 54"
    }

    // ---- the net ---------------------------------------------------------------
    Canvas {
        id: net
        anchors.top: cap.bottom; anchors.topMargin: 4
        anchors.bottom: readout.top; anchors.bottomMargin: 10
        width: parent.width
        renderStrategy: Canvas.Cooperative
        antialiasing: true

        // geometry (DnaPanel::geom)
        property real cell: 0
        property real head: 0
        property real tile: 0
        property real ox: 0
        property real oy: 0
        property real netw: 0
        property real neth: 0
        function geom() {
            const dw = width, dh = height, gp = 2
            const cw = (dw - gp * 3) / 12
            let c = Math.floor(Math.min(cw, dh / (9 + 3 * 0.40)) * 2) / 2
            const hd = Math.max(Math.round(c * 0.40), 8)
            c = Math.max(1, Math.floor(Math.min(cw, (dh - hd * 3) / 9) * 2) / 2)
            cell = c; head = hd; tile = c * 0.80
            netw = c * 12 + gp * 3
            neth = (c * 3 + hd) * 3
            ox = Math.round((dw - netw) / 2)
            oy = Math.round((dh - neth) / 2)
        }
        onWidthChanged: geom()
        onHeightChanged: geom()

        function faceBox(fi) {
            const f = panel.info.net[fi]
            return { x: ox + f.fc * (cell * 3 + 2), y: oy + f.fr * (cell * 3 + head), w: cell * 3, h: head + cell * 3 }
        }
        function cellCenter(idx) {
            const b = faceBox(Math.floor(idx / 9)), ci = idx % 9
            return [b.x + ((ci % 3) + 0.5) * cell, b.y + head + (Math.floor(ci / 3) + 0.5) * cell]
        }
        function faceCenter(fi) {
            const b = faceBox(fi)
            return [b.x + cell * 1.5, b.y + head + cell * 1.5]
        }

        // ---- colour helpers ----
        function rgb(hex) { return [parseInt(hex.substr(1, 2), 16), parseInt(hex.substr(3, 2), 16), parseInt(hex.substr(5, 2), 16)] }
        function rgba(c, a) { return "rgba(" + c[0] + "," + c[1] + "," + c[2] + "," + a + ")" }
        function pal(key) { return rgb(panel.info.palette[key]) }

        function rr(ctx, x, y, w, h, r) {
            ctx.beginPath()
            ctx.moveTo(x + r, y)
            ctx.arcTo(x + w, y, x + w, y + h, r)
            ctx.arcTo(x + w, y + h, x, y + h, r)
            ctx.arcTo(x, y + h, x, y, r)
            ctx.arcTo(x, y, x + w, y, r)
            ctx.closePath()
        }
        // One bevelled sticker tile, centred at (cx, cy), optionally rotated.
        function tileAt(ctx, key, cx, cy, s, rot) {
            const c = pal(key)
            const top = c.map(v => Math.min(255, Math.round(v * 1.13 + 8)))
            const bot = c.map(v => Math.round(v * 0.84))
            ctx.save()
            ctx.translate(cx, cy)
            if (rot) ctx.rotate(rot)
            const h = s / 2, r = Math.max(1.5, s * 0.10)
            const g = ctx.createLinearGradient(0, -h, 0, h)
            g.addColorStop(0, rgba(top, 1)); g.addColorStop(0.44, rgba(c, 1)); g.addColorStop(1, rgba(bot, 1))
            rr(ctx, -h, -h, s, s, r)
            ctx.fillStyle = g
            ctx.fill()
            ctx.lineWidth = Math.max(1, s * 0.045)
            ctx.strokeStyle = "rgba(0,0,0,0.45)"
            ctx.stroke()
            ctx.beginPath()
            ctx.moveTo(-h + s * 0.2, -h + Math.max(1, s * 0.055))
            ctx.lineTo(-h + s * 0.8, -h + Math.max(1, s * 0.055))
            ctx.lineWidth = Math.max(1, s * 0.035)
            ctx.strokeStyle = "rgba(255,255,255,0.34)"
            ctx.stroke()
            ctx.restore()
        }
        function header(ctx, b, fi, hot) {
            const f = panel.info.net[fi]
            const fs = Math.max(7.5, Math.min(10, cell * 0.36))
            const mid = b.y + head * 0.5 + 0.5
            ctx.textBaseline = "middle"
            ctx.font = fs.toFixed(1) + "px 'DejaVu Sans Mono'"
            ctx.textAlign = "left"
            ctx.fillStyle = hot ? "#f0c070" : "rgba(198,212,240,0.66)"
            ctx.fillText(f.key, b.x + 4, mid)
            ctx.font = Math.max(fs - 1.4, 6.5).toFixed(1) + "px 'DejaVu Sans Mono'"
            ctx.textAlign = "right"
            ctx.fillStyle = "#3f4657"
            ctx.fillText(f.tag, b.x + b.w - 4, mid)
        }
        function arrowHead(ctx, x, y, dx, dy, len, col) {
            const l = Math.max(1e-6, Math.hypot(dx, dy)); dx /= l; dy /= l
            const px = -dy, py = dx, w = len * 0.44
            ctx.beginPath()
            ctx.moveTo(x - dx * len + px * w, y - dy * len + py * w); ctx.lineTo(x, y)
            ctx.lineTo(x - dx * len - px * w, y - dy * len - py * w)
            ctx.strokeStyle = col; ctx.lineWidth = 1; ctx.stroke()
        }

        onPaint: {
            const ctx = getContext("2d")
            ctx.reset()
            const d = panel.dna
            if (!d || !panel.info || cell < 1) return
            const k = d.k

            // 0 - plate: sheet brackets, face frames, headers, resting tiles
            const fx = ox - 7, fy = oy - 7, fw = netw + 14, fh = neth + 14, bl = 8
            ctx.strokeStyle = "rgba(158,176,214,0.20)"; ctx.lineWidth = 1
            ctx.beginPath()
            ctx.moveTo(fx, fy + bl); ctx.lineTo(fx, fy); ctx.lineTo(fx + bl, fy)
            ctx.moveTo(fx + fw - bl, fy); ctx.lineTo(fx + fw, fy); ctx.lineTo(fx + fw, fy + bl)
            ctx.moveTo(fx + fw, fy + fh - bl); ctx.lineTo(fx + fw, fy + fh); ctx.lineTo(fx + fw - bl, fy + fh)
            ctx.moveTo(fx + bl, fy + fh); ctx.lineTo(fx, fy + fh); ctx.lineTo(fx, fy + fh - bl)
            ctx.stroke()
            for (let fi = 0; fi < 6; ++fi) {
                const b = faceBox(fi)
                rr(ctx, b.x + 0.5, b.y + 0.5, b.w - 1, b.h - 1, 2.5)
                ctx.fillStyle = "rgba(146,170,214,0.026)"; ctx.fill()
                ctx.strokeStyle = "rgba(158,176,214,0.16)"; ctx.stroke()
                header(ctx, b, fi, false)
                for (let i = 0; i < 9; ++i) {
                    const key = d.sheet[fi * 9 + i]
                    if (!key) continue
                    const c = cellCenter(fi * 9 + i)
                    tileAt(ctx, key, c[0], c[1], tile, 0)
                }
            }

            const fl = d.inflight
            // 1 - the turned face: lit plate, amber key, circular arrow, sockets
            if (fl) {
                const b = faceBox(fl.face)
                rr(ctx, b.x + 0.5, b.y + 0.5, b.w - 1, b.h - 1, 2.5)
                ctx.fillStyle = "rgba(240,192,112,0.045)"; ctx.fill()
                ctx.strokeStyle = "rgba(240,192,112,0.44)"; ctx.lineWidth = 1; ctx.stroke()
                header(ctx, b, fl.face, true)
                const gx = b.x + b.w - head * 1.9, gy = b.y + head * 0.5 + 0.5, gr = Math.max(2.6, head * 0.30)
                const dir = fl.cw ? 1 : -1
                const a0 = Math.PI * (fl.cw ? 0.75 : 1.25), a1 = Math.PI * (fl.cw ? 2.35 : -0.35)
                ctx.beginPath(); ctx.arc(gx, gy, gr, a0, a1, !fl.cw)
                ctx.strokeStyle = "rgba(240,192,112,0.78)"; ctx.stroke()
                arrowHead(ctx, gx + Math.cos(a1) * gr, gy + Math.sin(a1) * gr, -Math.sin(a1) * dir, Math.cos(a1) * dir, gr * 1.1, "rgba(240,192,112,0.85)")
                ctx.setLineDash([2, 3])
                for (const i of fl.moved) {
                    const c = cellCenter(i)
                    rr(ctx, c[0] - tile / 2, c[1] - tile / 2, tile, tile, 2)
                    ctx.fillStyle = "rgba(6,7,10,0.74)"; ctx.fill()
                    ctx.strokeStyle = "rgba(158,176,214,0.22)"; ctx.stroke()
                }
                ctx.setLineDash([])

                // 2 - permutation arcs
                const centre = faceCenter(fl.face)
                const n = 14
                const geo = []
                for (const t of fl.tiles) {
                    const s = cellCenter(t[0]), dd = cellCenter(t[1])
                    const g = { key: t[2], s: s, d: dd, arc: t[3], rs: 0, a0: 0, sw: 0, qx: 0, qy: 0, pts: [] }
                    if (g.arc) {
                        const vx = s[0] - centre[0], vy = s[1] - centre[1]
                        g.rs = Math.hypot(vx, vy)
                        g.a0 = Math.atan2(vy, vx)
                        const T = Math.PI * 2
                        const sw0 = (((Math.atan2(dd[1] - centre[1], dd[0] - centre[0]) - g.a0) % T) + T) % T
                        g.sw = fl.cw ? sw0 : sw0 - T
                    } else {
                        const mx = (s[0] + dd[0]) / 2, my = (s[1] + dd[1]) / 2
                        const len = Math.max(1, Math.hypot(dd[0] - s[0], dd[1] - s[1]))
                        if (len > cell * 5) {
                            // The B/L seam: leave through the nearest sheet edge and return.
                            let outY = my < oy + neth * 0.5 ? oy - cell * 0.55 : oy + neth + cell * 0.55
                            outY = Math.max(4, Math.min(height - 4, outY))
                            g.qx = mx; g.qy = 2 * outY - my
                        } else {
                            const tx = centre[1] - my, ty = mx - centre[0]
                            const tl = Math.max(1, Math.hypot(tx, ty))
                            const bow = 0.16 * len * (fl.cw ? 1 : -1)
                            g.qx = mx + tx / tl * bow; g.qy = my + ty / tl * bow
                        }
                    }
                    for (let j = 0; j <= n; ++j) {
                        const u = j / n, iu = 1 - u
                        if (g.arc) {
                            const an = g.a0 + g.sw * u
                            g.pts.push([centre[0] + Math.cos(an) * g.rs, centre[1] + Math.sin(an) * g.rs])
                        } else {
                            g.pts.push([iu * iu * s[0] + 2 * iu * u * g.qx + u * u * dd[0],
                                        iu * iu * s[1] + 2 * iu * u * g.qy + u * u * dd[1]])
                        }
                    }
                    geo.push(g)
                }
                const poly = (pts, upto) => {
                    ctx.moveTo(pts[0][0], pts[0][1])
                    for (let j = 1; j <= upto; ++j) ctx.lineTo(pts[j][0], pts[j][1])
                }
                ctx.lineCap = "round"; ctx.lineJoin = "round"
                ctx.beginPath()
                for (const g of geo) poly(g.pts, n)
                ctx.strokeStyle = "rgba(9,10,14,0.86)"; ctx.lineWidth = 3.4; ctx.stroke()
                const upto = Math.max(1, Math.min(14, Math.round(k * 14)))
                for (const g of geo) {
                    if (!g.key) continue
                    const c = pal(g.key)
                    rr(ctx, g.d[0] - tile / 2, g.d[1] - tile / 2, tile, tile, 2)
                    ctx.fillStyle = rgba(c, 0.10 + 0.16 * k); ctx.fill()
                    ctx.strokeStyle = rgba(c, 0.28 + 0.34 * k); ctx.lineWidth = 1; ctx.stroke()
                    ctx.beginPath(); poly(g.pts, n)
                    ctx.strokeStyle = rgba(c, 0.30); ctx.lineWidth = 1; ctx.stroke()
                    ctx.beginPath(); poly(g.pts, upto)
                    ctx.strokeStyle = rgba(c, 0.80); ctx.lineWidth = 1.1; ctx.stroke()
                }

                // 3 - travelling tiles, each under a soft white halo
                for (const g of geo) {
                    let tp, rot = 0
                    if (g.arc) {
                        const an = g.a0 + g.sw * k
                        tp = [centre[0] + Math.cos(an) * g.rs, centre[1] + Math.sin(an) * g.rs]
                        rot = g.sw * k
                    } else {
                        const iu = 1 - k
                        tp = [iu * iu * g.s[0] + 2 * iu * k * g.qx + k * k * g.d[0],
                              iu * iu * g.s[1] + 2 * iu * k * g.qy + k * k * g.d[1]]
                    }
                    const gr = tile * 1.7 / 2
                    const halo = ctx.createRadialGradient(tp[0], tp[1], 0, tp[0], tp[1], gr)
                    halo.addColorStop(0, "rgba(236,244,255,0.42)")
                    halo.addColorStop(0.42, "rgba(214,232,255,0.17)")
                    halo.addColorStop(1, "rgba(200,224,255,0)")
                    ctx.fillStyle = halo
                    ctx.fillRect(tp[0] - gr, tp[1] - gr, gr * 2, gr * 2)
                    if (g.key) tileAt(ctx, g.key, tp[0], tp[1], tile, rot)
                }
            }

            // 4 - commit pulse
            if (d.pulse.t < 1) {
                const e = 1 - d.pulse.t
                for (const ci of d.pulse.cells) {
                    const p = cellCenter(ci)
                    rr(ctx, p[0] - tile / 2, p[1] - tile / 2, tile, tile, 2)
                    ctx.fillStyle = "rgba(232,242,255," + (0.26 * e * e) + ")"; ctx.fill()
                    ctx.strokeStyle = "rgba(232,242,255," + (0.62 * e) + ")"; ctx.lineWidth = 1; ctx.stroke()
                }
            }

            // 5 - solved: the whole sheet lights once, then stands down
            if (d.flash < 1.9) {
                const e2 = Math.sin(Math.PI * Math.max(0, Math.min(1, d.flash / 1.9)))
                const x = ox - 8, y = oy - 8, w = netw + 16, h = neth + 16
                for (const [lw, a] of [[9, 0.06], [5, 0.14], [1, 0.62]]) {
                    rr(ctx, x, y, w, h, 4)
                    ctx.lineWidth = lw * Math.max(1, e2)
                    ctx.strokeStyle = "rgba(70,224,140," + (a * e2) + ")"; ctx.stroke()
                }
                for (let fi = 0; fi < 6; ++fi) {
                    const b = faceBox(fi)
                    rr(ctx, b.x + 0.5, b.y + 0.5, b.w - 1, b.h - 1, 2.5)
                    ctx.fillStyle = "rgba(70,224,140," + (0.13 * e2) + ")"; ctx.fill()
                }
            }

            // 6 - inspection outline (the tooltip is a QML item above)
            if (d.inspect) {
                const p = cellCenter(d.inspect.idx)
                rr(ctx, p[0] - tile / 2 - 1.5, p[1] - tile / 2 - 1.5, tile + 3, tile + 3, 2.5)
                ctx.strokeStyle = "rgba(232,242,255,0.85)"; ctx.lineWidth = 1; ctx.stroke()
            }
        }

        MouseArea {
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: containsMouse && panel.dna && panel.dna.inspect ? Qt.CrossCursor : Qt.ArrowCursor
            function cellAt(x, y) {
                for (let fi = 0; fi < 6; ++fi) {
                    const b = net.faceBox(fi)
                    if (x < b.x || x > b.x + b.w || y < b.y + net.head || y > b.y + b.h) continue
                    const cc = Math.floor((x - b.x) / net.cell), rr = Math.floor((y - b.y - net.head) / net.cell)
                    if (cc < 0 || cc > 2 || rr < 0 || rr > 2) continue
                    return fi * 9 + rr * 3 + cc
                }
                return -1
            }
            onPositionChanged: (m) => Rubik.setHover(cellAt(m.x, m.y))
            onExited: Rubik.setHover(-1)
        }

        // Hover inspector tooltip.
        Rectangle {
            id: tip
            readonly property var ins: panel.dna ? panel.dna.inspect : null
            readonly property var p: ins ? net.cellCenter(ins.idx) : [0, 0]
            visible: ins !== null
            width: tipCol.width + 12; height: tipCol.height + 9
            radius: 2
            color: Qt.rgba(9/255, 11/255, 16/255, 0.92)
            border.color: Qt.rgba(158/255, 176/255, 214/255, 0.30)
            x: { let bx = p[0] + net.tile * 0.8
                 if (bx + width > net.width - 2) bx = p[0] - net.tile * 0.8 - width
                 return Math.max(2, bx) }
            y: { let by = p[1] - height - net.tile * 0.5
                 if (by < 2) by = p[1] + net.tile * 0.6
                 return Math.min(net.height - height - 2, by) }
            Column {
                id: tipCol
                x: 6; y: 4; spacing: 1.6
                Repeater {
                    model: 3
                    delegate: Label2 {
                        required property int index
                        readonly property string modelData: tip.ins ? tip.ins.lines[index] : ""
                        text: modelData; px: 8; color: index === 0 ? "#dfe7f6" : "#7c88a4"
                    }
                }
            }
        }

        // STATE VERIFIED badge
        Rectangle {
            readonly property real a: panel.dna && panel.dna.badge ? Math.min(1, panel.dna.flash / 0.34) : 0
            visible: a > 0
            opacity: a
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom; anchors.bottomMargin: 2
            width: badgeTxt.width + 14; height: badgeTxt.height + 5; radius: 2
            color: Qt.rgba(9/255, 14/255, 11/255, 0.78)
            border.color: Qt.rgba(70/255, 224/255, 140/255, 0.42)
            Label2 { id: badgeTxt; anchors.centerIn: parent; text: "STATE VERIFIED / SOLVED"; px: 7.5; ls: 0.2; color: th.ok }
        }
    }

    // ---- readout ------------------------------------------------------------
    Column {
        id: readout
        readonly property var r: panel.dna ? panel.dna.readout : null
        anchors.bottom: parent.bottom
        width: parent.width
        spacing: 0
        SectionHead { width: parent.width; leftText: "ACTIVE TRANSFORMATION"; rightText: readout.r ? readout.r.note : "" }
        Item { width: 1; height: 5 }
        Grid {
            width: parent.width
            columns: 4; rowSpacing: 3
            readonly property color vc: readout.r && readout.r.live ? th.val : "#4a5266"
            readonly property real half: (width - 120) / 2
            Label2 { width: 60; height: 15; verticalAlignment: Text.AlignVCenter; text: "MOVE"; px: 7.5; ls: 0.18; color: th.dimmer }
            Label2 { width: parent.half; height: 15; verticalAlignment: Text.AlignVCenter; text: readout.r ? readout.r.name : ""; px: 11; color: parent.vc }
            Label2 { width: 60; height: 15; verticalAlignment: Text.AlignVCenter; text: "AXIS"; px: 7.5; ls: 0.18; color: th.dimmer }
            Label2 { width: parent.half; height: 15; verticalAlignment: Text.AlignVCenter; horizontalAlignment: Text.AlignRight; text: readout.r ? readout.r.axis : ""; px: 8.5; color: parent.vc }
            Label2 { width: 60; height: 15; verticalAlignment: Text.AlignVCenter; text: "DIRECTION"; px: 7.5; ls: 0.18; color: th.dimmer }
            Label2 { width: parent.half; height: 15; verticalAlignment: Text.AlignVCenter; text: readout.r ? readout.r.dir : ""; px: 8.5; color: parent.vc }
            Label2 { width: 60; height: 15; verticalAlignment: Text.AlignVCenter; text: "ANGLE"; px: 7.5; ls: 0.18; color: th.dimmer }
            Label2 { width: parent.half; height: 15; verticalAlignment: Text.AlignVCenter; horizontalAlignment: Text.AlignRight; text: readout.r ? readout.r.angle : ""; px: 8.5; color: parent.vc }
        }
        Item { width: 1; height: 7 }
        Item {
            width: parent.width; height: 8
            Meter { width: parent.width - 47; height: 6; value: panel.dna ? panel.dna.k : 0; fill: Qt.rgba(1, 186/255, 92/255, 0.62) }
            Label2 { anchors.right: parent.right; y: -3; text: Math.round((panel.dna ? panel.dna.k : 0) * 100) + "%"; px: 9 }
        }
        Item { width: 1; height: 12 }
        SectionHead { width: parent.width; leftText: "CUBE INTEGRITY"; rightText: "placed · oriented" }
        Item { width: 1; height: 3 }
        Repeater {
            model: 3
            delegate: Item {
                required property int index
                readonly property var modelData: panel.dna ? panel.dna.integrity[index] : ["", "", 0]
                width: readout.width; height: 14
                Label2 { anchors.verticalCenter: parent.verticalCenter; text: modelData[0]; px: 7.5; ls: 0.16; color: th.dimmer }
                Meter { x: 58; anchors.verticalCenter: parent.verticalCenter; width: parent.width - 58 - 52; height: 6
                        value: modelData[2]; fill: Qt.rgba(70/255, 224/255, 140/255, 0.55) }
                Label2 { anchors.right: parent.right; anchors.verticalCenter: parent.verticalCenter; text: modelData[1]; px: 9 }
            }
        }
        Item { width: 1; height: 9 }
        Row {
            spacing: 8
            readonly property string st: panel.frame ? panel.frame.status.cube : ""
            readonly property color dot: st === "SOLVED" ? th.ok : st === "SCRAMBLED" ? th.idle : th.busy
            Rectangle { width: 12; height: 12; radius: 6; color: "transparent"; border.color: th.a(parent.dot, 0.3)
                Rectangle { anchors.centerIn: parent; width: 6; height: 6; radius: 3; color: parent.parent.dot } }
            Label2 { text: parent.st; px: 8.5; ls: 0.2 }
        }
    }

    Connections {
        target: panel
        function onFrameChanged() { net.requestPaint() }
    }
}
