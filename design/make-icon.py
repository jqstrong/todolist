#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
make-icon.py --- 「稿纸」Gaozhi 的应用图标。

界面在第二版里只剩一种颜色：靛蓝，而且只用在一件事上——完成。
图标就说这一件事：一块靛蓝的圆角方块，一个白色的勾。
一个记号、一个颜色，16px 时也还认得出。

（第一版图标是「稿纸 + 朱砂页边线 + 靛墨勾」两个记号。32px 时两个记号会互相压住，
  而纸与朱砂在新界面里也已经不存在了，所以换成现在这个。取舍见 docs/design-notes.md。）

做法：在 1024 的坐标里描述，按 SS 倍超采样绘制再 LANCZOS 缩小，
因此边缘是干净的抗锯齿，不需要任何 SVG 光栅化器。
确定性：没有随机、没有网络、没有外部素材。

用法：
  & "C:\\Users\\10269\\.dsh\\dsh-runtimes\\dsh-primary-runtime\\dependencies\\python\\python.exe" make-icon.py
"""

from __future__ import annotations

import os
from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))

TILE_TOP = (0x37, 0x60, 0xEE)
TILE_BOTTOM = (0x27, 0x49, 0xCE)
TICK = (0xFF, 0xFF, 0xFF)

CANVAS = 1024
SS = 4
CORNER_RADIUS = 224

TICK_P1 = (300, 440)
TICK_V = (455, 640)
TICK_P3 = (740, 350)
TICK_WIDTH = 150

SHEET_BG = (0x6E, 0x72, 0x76)
SHEET_SIZES = [256, 128, 64, 48, 32, 16]


def s(value):
    return int(round(value * SS))


def tile_layer(size):
    column = Image.new("RGB", (1, size))
    px = column.load()
    for y in range(size):
        t = y / max(1, size - 1)
        px[0, y] = tuple(
            int(round(TILE_TOP[i] + (TILE_BOTTOM[i] - TILE_TOP[i]) * t)) for i in range(3)
        )
    return column.resize((size, size)).convert("RGBA")


def tick_layer(size):
    layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer)
    width = s(TICK_WIDTH)
    half = width / 2.0
    points = [(s(x), s(y)) for x, y in (TICK_P1, TICK_V, TICK_P3)]
    draw.line(points, fill=TICK + (255,), width=width, joint="curve")
    for x, y in (points[0], points[-1]):
        draw.ellipse([x - half, y - half, x + half, y + half], fill=TICK + (255,))
    return layer


def build_master():
    size = CANVAS * SS
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        [0, 0, size - 1, size - 1], radius=s(CORNER_RADIUS), fill=255
    )
    icon = Image.alpha_composite(tile_layer(size), tick_layer(size))
    icon.putalpha(mask)
    return icon.resize((CANVAS, CANVAS), Image.LANCZOS)


def proof_sheet(master):
    pad, gap = 16, 18
    total = pad * 2 + sum(SHEET_SIZES) + gap * (len(SHEET_SIZES) - 1)
    height = pad * 2 + max(SHEET_SIZES)
    sheet = Image.new("RGB", (total, height), SHEET_BG)
    x = pad
    for sz in SHEET_SIZES:
        small = master.resize((sz, sz), Image.LANCZOS)
        sheet.paste(small, (x, pad + max(SHEET_SIZES) - sz), small)
        x += sz + gap
    return sheet


def main():
    master = build_master()
    master.save(os.path.join(HERE, "app-icon.png"))
    master.resize((32, 32), Image.LANCZOS).save(os.path.join(HERE, "icon-proof-32.png"))
    proof_sheet(master).save(os.path.join(HERE, "icon-proof-sheet.png"))

    left = TICK_P1[0] - TICK_WIDTH / 2.0
    right = TICK_P3[0] + TICK_WIDTH / 2.0
    top = TICK_P3[1] - TICK_WIDTH / 2.0
    bottom = TICK_V[1] + TICK_WIDTH / 2.0
    print("tile      : rounded square, radius %d (%.1f%%)" % (CORNER_RADIUS, 100.0 * CORNER_RADIUS / CANVAS))
    print("tick span : x %.0f..%.0f = %.1f%% of tile (%.1f px @32)" % (left, right, 100.0 * (right - left) / CANVAS, (right - left) / 32.0))
    print("tick box  : y %.0f..%.0f, centre %.1f%%" % (top, bottom, 100.0 * (top + bottom) / 2 / CANVAS))
    print("stroke    : %d (%.1f%%) -> %.1f px @32" % (TICK_WIDTH, 100.0 * TICK_WIDTH / CANVAS, TICK_WIDTH / 32.0))
    print("written   : app-icon.png, icon-proof-32.png, icon-proof-sheet.png")


if __name__ == "__main__":
    main()
