#!/usr/bin/env python3
"""Writes src/qml/studio.hdr: a small equirectangular HDR "photo studio" used as
the Qt Quick 3D light probe (image-based lighting + reflections).

Pure standard library (no numpy / Pillow): Radiance RGBE with new-style RLE
scanlines. Dark room, a large warm soft box in front of the cube (where the
engine's key light sits: dir ~ (0.26, 0.14, 0.955)), a cool strip light behind
for rim reflections, and a faint floor bounce.

    python3 tools/make_studio_hdr.py
"""
import math, os, struct

W, H = 512, 256

def smooth(e0, e1, x):
    t = max(0.0, min(1.0, (x - e0) / (e1 - e0)))
    return t * t * (3 - 2 * t)

def box(az, el, caz, cel, hw, hh, soft):
    # rounded-rectangle soft box in (azimuth, elevation) degrees
    daz = (az - caz + 180) % 360 - 180
    dx = abs(daz) - hw
    dy = abs(el - cel) - hh
    d = math.hypot(max(dx, 0), max(dy, 0)) + min(max(dx, dy), 0)
    return 1.0 - smooth(-soft, soft, d)

def radiance(az, el):
    # base: dark blue-grey room, brighter towards the ceiling
    t = (el + 90) / 180
    r = 0.010 + 0.040 * t ** 2
    g = 0.012 + 0.046 * t ** 2
    b = 0.018 + 0.070 * t ** 2
    # floor bounce
    if el < 0:
        f = 0.012 * smooth(-90, 0, el)
        r += f; g += f; b += f
    # key soft box (warm) in front-right, slightly high
    k = box(az, el, 16, 14, 22, 15, 4)
    r += 5.5 * k; g += 5.0 * k; b += 4.4 * k
    # fill box (neutral) front-left, lower and dimmer
    k = box(az, el, -42, 4, 16, 18, 6)
    r += 1.1 * k; g += 1.15 * k; b += 1.25 * k
    # cool rim strip behind
    k = box(az, el, 180, 22, 60, 4, 3)
    r += 1.2 * k; g += 1.8 * k; b += 3.2 * k
    # ceiling strip
    k = box(az, el, 0, 70, 160, 3, 3)
    r += 0.5 * k; g += 0.55 * k; b += 0.6 * k
    return r, g, b

def rgbe(r, g, b):
    v = max(r, g, b)
    if v < 1e-32:
        return (0, 0, 0, 0)
    m, e = math.frexp(v)
    s = m * 256.0 / v
    return (int(r * s), int(g * s), int(b * s), e + 128)

def rle_channel(vals):
    out = bytearray()
    i = 0
    while i < len(vals):
        n = min(128, len(vals) - i)
        out.append(n)
        out.extend(vals[i:i + n])
        i += n
    return out

def main():
    path = os.path.join(os.path.dirname(__file__), "..", "src", "qml", "studio.hdr")
    with open(path, "wb") as f:
        f.write(b"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n")
        f.write(f"-Y {H} +X {W}\n".encode())
        for y in range(H):
            el = 90 - (y + 0.5) / H * 180
            px = [rgbe(*radiance((x + 0.5) / W * 360 - 180, el)) for x in range(W)]
            f.write(bytes([2, 2, W >> 8, W & 255]))
            for c in range(4):
                f.write(rle_channel(bytes(p[c] for p in px)))
    print("wrote", os.path.normpath(path), os.path.getsize(path), "bytes")

if __name__ == "__main__":
    main()
