"""Génère le jeu d'icônes DualLink.

Le nom de l'application décrit deux liens : le marque est fait de deux
anneaux entrelacés (chain link). Les couleurs reprennent le dégradé du
titre de l'application: cyan -> violet sur fond sombre.

    python tools/gen_icons.py

Produit:
    src-tauri/icons/icon.png        512x512 RGBA (bundle, installateur)
    src-tauri/icons/icon.ico        multi-tailles (16 -> 256)
    src-tauri/icons/tray-icon.png   32x32 RGBA (barre des tâches)
"""

import io
import math
import struct
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parent.parent
ICONS = ROOT / "src-tauri" / "icons"

CYAN = (0, 212, 255, 255)
PURPLE = (170, 102, 255, 255)
BG_TOP = (26, 26, 62, 255)
BG_BOTTOM = (10, 10, 26, 255)

SS = 8  # suréchantillonnage: on dessine large, on réduit ensuite


def gradient_bg(w, h, radius):
    """Fond arrondi, dégradé vertical + halo cyan en haut à droite."""
    base = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    grad = Image.new("RGBA", (w, h))
    d = ImageDraw.Draw(grad)
    for y in range(h):
        t = y / max(1, h - 1)
        d.line(
            [(0, y), (w, y)],
            fill=tuple(round(a + (b - a) * t) for a, b in zip(BG_TOP, BG_BOTTOM)),
        )

    glow = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    ImageDraw.Draw(glow).ellipse(
        [w * 0.45, -h * 0.35, w * 1.35, h * 0.65], fill=(0, 212, 255, 60)
    )
    glow = glow.filter(ImageFilter.GaussianBlur(w * 0.10))
    grad = Image.alpha_composite(grad, glow)

    mask = Image.new("L", (w, h), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, w - 1, h - 1], radius=radius, fill=255)
    base.paste(grad, (0, 0), mask)
    return base


def _ring(draw, cx, cy, r, stroke, color):
    draw.ellipse([cx - r, cy - r, cx + r, cy + r], outline=color, width=stroke)


def _disc(size, cx, cy, r):
    m = Image.new("L", size, 0)
    ImageDraw.Draw(m).ellipse([cx - r, cy - r, cx + r, cy + r], fill=255)
    return m


def _intersections(c1, c2, r):
    """Points d'intersection de deux cercles de même rayon."""
    (x1, y1), (x2, y2) = c1, c2
    dx, dy = x2 - x1, y2 - y1
    dist = math.hypot(dx, dy)
    if dist == 0 or dist > 2 * r:
        return []

    a = dist / 2
    h = math.sqrt(max(0.0, r * r - a * a))
    mx, my = (x1 + x2) / 2, (y1 + y2) / 2
    ux, uy = -dy / dist, dx / dist
    return [(mx + h * ux, my + h * uy), (mx - h * ux, my - h * uy)]


def links(size):
    """Deux anneaux entrelacés, transparents, RGBA."""
    s = size * SS
    stroke = max(1, int(s * 0.080))
    r = int(s * 0.215)
    c1 = (s * 0.355, s * 0.545)
    c2 = (s * 0.645, s * 0.455)

    mask_a = Image.new("L", (s, s), 0)
    _ring(ImageDraw.Draw(mask_a), c1[0], c1[1], r, stroke, 255)
    mask_b = Image.new("L", (s, s), 0)
    _ring(ImageDraw.Draw(mask_b), c2[0], c2[1], r, stroke, 255)

    # Entrelacs: au croisement du haut l'anneau A passe au-dessus, au
    # croisement du bas c'est B. On évide donc la Masque qui passe dessous.
    pts = _intersections(c1, c2, r)
    gap = stroke * 0.55

    if len(pts) == 2:
        top, bottom = pts[0], pts[1]
        if top[1] < bottom[1]:
            top, bottom = bottom, top

        cut_a_under_b = Image.new("L", (s, s), 0)
        ImageDraw.Draw(cut_a_under_b).ellipse(
            [bottom[0] - gap, bottom[1] - gap, bottom[0] + gap, bottom[1] + gap], fill=255
        )
        cut_b_under_a = Image.new("L", (s, s), 0)
        ImageDraw.Draw(cut_b_under_a).ellipse(
            [top[0] - gap, top[1] - gap, top[0] + gap, top[1] + gap], fill=255
        )
        mask_a = Image.composite(Image.new("L", (s, s), 0), mask_a, cut_a_under_b)
        mask_b = Image.composite(Image.new("L", (s, s), 0), mask_b, cut_b_under_a)

    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))

    # Les anneaux sont teintés du cyan au violet en diagonale.
    grad = Image.new("RGBA", (s, s))
    gd = ImageDraw.Draw(grad)
    for y in range(s):
        for_x = y / max(1, s - 1)
        gd.line(
            [(0, y), (s, y)],
            fill=tuple(round(a + (b - a) * for_x) for a, b in zip(CYAN, PURPLE)),
        )

    img.paste(grad, (0, 0), mask_a)
    layer_b = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    layer_b.paste(grad, (0, 0), mask_b)
    img = Image.alpha_composite(img, layer_b)

    return img.resize((size, size), Image.LANCZOS)


def badge(size):
    """Icône applicative: badge arrondi + marque."""
    s = size * SS
    bg = gradient_bg(s, s, radius=int(s * 0.22))
    mark = links(s)
    mark = mark.resize((s, s), Image.LANCZOS)
    return Image.alpha_composite(bg, mark).resize((size, size), Image.LANCZOS)


def flat(size):
    """Marque seule, sans fond: lisible en barre des tâches."""
    return links(size)


def write_ico(path, dims):
    """Écrit un .ico multi-tailles.

    PIL n'embarque qu'une seule taille ici: son `append_images` ne retient
    que les images dont la taille correspond exactement à une entrée de
    `sizes`, et l'image de base impose la première. On assemble donc le
    conteneur ICO à la main, comme le fait un éditeur d'icônes.
    """
    frames = []
    for n in dims:
        img = (flat(n) if n < 48 else badge(n)).convert("RGBA")
        frames.append((n, img))

    offset = 6 + 16 * len(frames)
    header = struct.pack("<HHH", 0, 1, len(frames))
    entries, blobs = b"", b""

    for n, img in frames:
        # PNG dans l'ICO: compact et conservé tel quel (pas de perte alpha).
        buf = io.BytesIO()
        img.save(buf, format="PNG")
        data = buf.getvalue()
        # 0 = couleur 256 (256x256), 0 = truecolor + alpha
        entries += struct.pack(
            "<BBBBHHII",
            0 if n == 256 else n,
            0 if n == 256 else n,
            0,
            0,
            1,
            32,
            len(data),
            offset,
        )
        blobs += data
        offset += len(data)

    path.write_bytes(header + entries + blobs)
    return len(frames)


def main():
    ICONS.mkdir(parents=True, exist_ok=True)

    app = badge(512)
    app.save(ICONS / "icon.png")
    print("icon.png      512x512 RGBA")

    tray = flat(32)
    tray.save(ICONS / "tray-icon.png")
    print("tray-icon.png  32x32 RGBA")

    # Sous 48px le badge devient illisible: on garde la marque seule.
    # Au-delà, le badge donne plus de présence (installateur, menu).
    dims = [16, 20, 24, 32, 40, 48, 64, 128, 256]
    n = write_ico(ICONS / "icon.ico", dims)
    print(f"icon.ico      {n} tailles ({', '.join(str(d) for d in dims)})")


if __name__ == "__main__":
    main()