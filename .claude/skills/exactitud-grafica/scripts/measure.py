"""Mide la forma de una imagen en unidades del render y compara referencia contra render.

Uso:
  python measure.py stats IMG --shape EXPR --anchor EXPR --anchor-size WxH [opciones]
  python measure.py compare REF RENDER --shape EXPR --anchor EXPR --anchor-size WxH
                    [--render-shape EXPR] [--render-anchor EXPR] [--diff out.png]
  python measure.py mask IMG --shape EXPR [--out mask.png]

EXPR es una expresión numpy sobre los canales r, g, b, a (0-255), por ejemplo
  "(r > 110) & (r > 2.2 * g) & (r > 1.6 * b)"     rojo del pelo
  "(g > 150) & (r < 120) & (b < 120)"             verde de la placa
El ancla es un elemento de tamaño conocido en unidades del render (en Capri, la placa:
8.08x3.308 R). Con ella se pasa de píxeles a unidades y se fija el origen en su centro.
Opciones:
  --anchor-align center|bottom|top  de dónde sale el centro del ancla en y (por defecto center;
                                    bottom si el flequillo tapa su borde de arriba)
  --close PX       cierre morfológico de la forma, para tapar brillos/manchas (por defecto 2)
  --core R         radio de apertura (en unidades) para separar el núcleo de las púas (4)
  --region X0:X1,Y0:Y1  solo cuenta la forma dentro de esa caja (unidades); sirve para descartar
                   cuerpo, fondo o texto que comparten color con la forma (p. ej. ",-6:")
  --json           salida en JSON
Coordenadas de salida: x a la derecha, y hacia arriba, origen en el centro del ancla.
"""

import argparse
import json
import math
import sys

import numpy as np
from PIL import Image
from scipy import ndimage as ndi
from scipy.signal import find_peaks
from scipy.spatial import ConvexHull


def load(path):
    im = Image.open(path).convert("RGBA")
    a = np.asarray(im).astype(np.int32)
    return {"r": a[..., 0], "g": a[..., 1], "b": a[..., 2], "a": a[..., 3]}


def evaluate(expr, ch):
    env = {"np": np, **ch}
    m = eval(expr, {"__builtins__": {}}, env)  # herramienta local: la expresión la escribe quien la usa
    return np.asarray(m, dtype=bool)


def largest(mask):
    lab, n = ndi.label(mask)
    if n == 0:
        sys.exit("ancla: ningún píxel coincide con la expresión")
    sizes = ndi.sum(mask, lab, range(1, n + 1))
    return lab == (1 + int(np.argmax(sizes)))


def frame(anchor_mask, size, align):
    """Escala (px por unidad) y centro del ancla en píxeles."""
    ys, xs = np.nonzero(largest(anchor_mask))
    w, h = size
    s = (xs.max() - xs.min() + 1) / w
    cx = (xs.min() + xs.max()) / 2
    if align == "bottom":
        cy = ys.max() - h * s / 2
    elif align == "top":
        cy = ys.min() + h * s / 2
    else:
        cy = (ys.min() + ys.max()) / 2
    measured_h = (ys.max() - ys.min() + 1) / s
    return s, (cx, cy), measured_h


def disk(radius_px):
    r = max(1, int(round(radius_px)))
    y, x = np.ogrid[-r : r + 1, -r : r + 1]
    return x * x + y * y <= r * r


def opening(mask, radius_px):
    """Apertura con un disco de cualquier radio, vía transformadas de distancia (rápida)."""
    eroded = ndi.distance_transform_edt(mask) > radius_px
    return ndi.distance_transform_edt(~eroded) <= radius_px


def shape_mask(ch, expr, close_px):
    m = evaluate(expr, ch)
    if close_px > 0:
        m = ndi.binary_closing(m, structure=disk(close_px))
    return ndi.binary_fill_holes(m) if close_px > 0 else m


def clip(mask, s, c, region):
    """Deja solo la parte de `mask` dentro de la caja `region` (unidades, extremos opcionales)."""
    if not region:
        return mask
    def span(part):
        lo, _, hi = part.partition(":")
        return (float(lo) if lo.strip() else -math.inf, float(hi) if hi.strip() else math.inf)

    parts = (region.split(",") + [""])[:2]
    (x0, x1), (y0, y1) = span(parts[0]), span(parts[1])
    h, w = mask.shape
    yy, xx = np.mgrid[0:h, 0:w]
    ux, uy = (xx - c[0]) / s, (c[1] - yy) / s
    return mask & (ux >= x0) & (ux <= x1) & (uy >= y0) & (uy <= y1)


def to_units(mask, s, c):
    ys, xs = np.nonzero(mask)
    return (xs - c[0]) / s, (c[1] - ys) / s


def stats(mask, s, c, core_r):
    x, y = to_units(mask, s, c)
    area = mask.sum() / s**2
    x0, x1, y0, y1 = x.min(), x.max(), y.min(), y.max()
    pts = np.column_stack([x, y])
    hull = ConvexHull(pts[:: max(1, len(pts) // 20000)])
    core = opening(mask, core_r * s)
    # Perfil polar alrededor del centroide: radio máximo por grado.
    cxm, cym = x.mean(), y.mean()
    ang = (np.degrees(np.arctan2(y - cym, x - cxm)) + 360) % 360
    rad = np.hypot(x - cxm, y - cym)
    prof = np.zeros(360)
    np.maximum.at(prof, ang.astype(int) % 360, rad)
    wrap = np.concatenate([prof[-30:], prof, prof[:30]])
    peaks, _ = find_peaks(wrap, prominence=0.04 * prof.max(), distance=4)
    tips = sorted({int(p - 30) % 360 for p in peaks if 30 <= p < 390})
    sectors = {f"{a}-{a + 30}": round(float(prof[a : a + 30].max()), 1) for a in range(0, 360, 30)}
    above = (y > 0).mean()
    return {
        "px_per_unit": round(float(s), 3),
        "extent": {"x": [round(float(x0), 1), round(float(x1), 1)], "y": [round(float(y0), 1), round(float(y1), 1)]},
        "size": [round(float(x1 - x0), 1), round(float(y1 - y0), 1)],
        "aspect_w_h": round(float((x1 - x0) / (y1 - y0)), 2),
        "area": round(float(area), 1),
        "centroid": [round(float(cxm), 2), round(float(cym), 2)],
        "bbox_fill": round(float(area / ((x1 - x0) * (y1 - y0))), 2),
        "hull_ratio": round(float(area / hull.volume), 2),
        "core_fraction": round(float(core.sum() / mask.sum()), 2),
        "mass_above_anchor_center": round(float(above), 2),
        "tips_deg": tips,
        "tip_count": len(tips),
        "max_radius_by_sector": sectors,
    }


def resample(mask, s, c, s_to, c_to, shape_to):
    """Lleva `mask` (escala s, centro c) al marco de otra imagen (escala s_to, centro c_to)."""
    h, w = shape_to
    yy, xx = np.mgrid[0:h, 0:w]
    ux, uy = (xx - c_to[0]) / s_to, (c_to[1] - yy) / s_to
    px, py = np.round(c[0] + ux * s).astype(int), np.round(c[1] - uy * s).astype(int)
    ok = (px >= 0) & (px < mask.shape[1]) & (py >= 0) & (py < mask.shape[0])
    out = np.zeros(shape_to, bool)
    out[ok] = mask[py[ok], px[ok]]
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("cmd", choices=["stats", "compare", "mask"])
    ap.add_argument("images", nargs="+")
    ap.add_argument("--shape", required=True)
    ap.add_argument("--anchor")
    ap.add_argument("--anchor-size")
    ap.add_argument("--anchor-align", default="center", choices=["center", "bottom", "top"])
    ap.add_argument("--render-shape")
    ap.add_argument("--render-anchor")
    ap.add_argument("--render-anchor-align")
    ap.add_argument("--close", type=float, default=2)
    ap.add_argument("--core", type=float, default=4)
    ap.add_argument("--region")
    ap.add_argument("--diff")
    ap.add_argument("--out")
    ap.add_argument("--json", action="store_true")
    a = ap.parse_args()

    if a.cmd == "mask":
        m = shape_mask(load(a.images[0]), a.shape, a.close)
        out = a.out or "mask.png"
        Image.fromarray((m * 255).astype(np.uint8)).save(out)
        print(f"{out}: {int(m.sum())} px")
        return

    if not (a.anchor and a.anchor_size):
        sys.exit("stats y compare necesitan --anchor y --anchor-size WxH")
    size = tuple(float(v) for v in a.anchor_size.lower().split("x"))

    ch = load(a.images[0])
    s, c, mh = frame(evaluate(a.anchor, ch), size, a.anchor_align)
    m = clip(shape_mask(ch, a.shape, a.close), s, c, a.region)

    if a.cmd == "stats":
        st = stats(m, s, c, a.core)
        st["anchor_height_measured"] = round(float(mh), 2)
        if a.json:
            print(json.dumps(st, indent=2))
        else:
            for k, v in st.items():
                print(f"{k:28} {v}")
        return

    if len(a.images) != 2:
        sys.exit("compare necesita REF y RENDER")
    ch2 = load(a.images[1])
    s2, c2, _ = frame(
        evaluate(a.render_anchor or a.anchor, ch2), size, a.render_anchor_align or a.anchor_align
    )
    m2 = shape_mask(ch2, a.render_shape or a.shape, a.close)
    ref = m
    ren = clip(resample(m2, s2, c2, s, c, m.shape), s, c, a.region)
    inter, union = (ref & ren).sum(), (ref | ren).sum()
    res = {
        "iou": round(float(inter / union), 3),
        "recall": round(float(inter / ref.sum()), 3),
        "precision": round(float(inter / max(ren.sum(), 1)), 3),
        "area_ref": round(float(ref.sum() / s**2), 1),
        "area_render": round(float(ren.sum() / s**2), 1),
    }
    if a.diff:
        vis = np.zeros((*ref.shape, 3), np.uint8)
        vis[ref & ren] = (200, 200, 200)
        vis[ref & ~ren] = (255, 60, 60)  # falta en el render
        vis[ren & ~ref] = (60, 140, 255)  # sobra en el render
        Image.fromarray(vis).save(a.diff)
        res["diff"] = a.diff
    print(json.dumps(res, indent=2) if a.json else "\n".join(f"{k:12} {v}" for k, v in res.items()))


if __name__ == "__main__":
    main()
