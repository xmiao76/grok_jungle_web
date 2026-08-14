"""Downscale desktop portraits/terrain and copy Cinzel into public/assets."""

from __future__ import annotations

import os
import shutil
from pathlib import Path

from PIL import Image

REPO = Path(__file__).resolve().parents[1]
DESKTOP = Path(os.environ.get("GROK_JUNGLE", REPO.parent / "grok_jungle"))
SRC = DESKTOP / "src" / "jungle" / "assets"
DST = REPO / "public" / "assets"

ANIMALS = ("rat", "cat", "dog", "wolf", "leopard", "tiger", "lion", "elephant")
SIDES = ("gold", "scarlet")
TERRAIN = ("den", "land", "trap", "water")


def _save_png(im: Image.Image, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    im.save(dest, "PNG", optimize=True)


def prep_pieces(size: int = 256) -> None:
    for side in SIDES:
        for animal in ANIMALS:
            src = SRC / "pieces" / side / f"{animal}.png"
            if not src.is_file():
                raise FileNotFoundError(src)
            im = Image.open(src).convert("RGBA")
            im = im.resize((size, size), Image.Resampling.LANCZOS)
            _save_png(im, DST / "pieces" / side / f"{animal}.png")
            print(f"piece {side}/{animal}.png {src.stat().st_size} -> {im.size}")


def prep_terrain(size: int = 192) -> None:
    for name in TERRAIN:
        src = SRC / "terrain" / f"{name}.png"
        if not src.is_file():
            raise FileNotFoundError(src)
        im = Image.open(src).convert("RGBA")
        im = im.resize((size, size), Image.Resampling.LANCZOS)
        _save_png(im, DST / "terrain" / f"{name}.png")
        print(f"terrain {name}.png {im.size}")


def prep_font() -> None:
    src = SRC / "fonts" / "Cinzel.ttf"
    dest = DST / "fonts" / "Cinzel.ttf"
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dest)
    print(f"font Cinzel.ttf {src.stat().st_size} bytes")


def main() -> None:
    if not (SRC / "pieces" / "gold" / "rat.png").is_file():
        raise SystemExit(f"desktop assets not found at {SRC}")
    prep_pieces()
    prep_terrain()
    prep_font()
    print(f"wrote assets under {DST}")


if __name__ == "__main__":
    main()
