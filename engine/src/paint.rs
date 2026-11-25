//! 448×576 RGBA8 board. Every pixel is opaque.

pub const FRAME_W: usize = 448;
pub const FRAME_H: usize = 576;
pub const CELL: usize = 64;
const SPRITE: usize = 52;

const RAW: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sprites.bin"));

#[derive(Clone, Copy)]
pub struct SpritePiece {
    pub color: u8,
    pub kind: u8,
}

pub struct Frame {
    pub cells: [Option<SpritePiece>; 63],
    pub last_src: Option<u8>,
    pub last_dst: Option<u8>,
    pub captured: Option<SpritePiece>,
    pub phase: &'static str,
    pub winner: &'static str,
    pub reason: &'static str,
}

impl Frame {
    pub fn empty() -> Self {
        Self {
            cells: [None; 63],
            last_src: None,
            last_dst: None,
            captured: None,
            phase: "playing",
            winner: "",
            reason: "",
        }
    }
}

pub fn render(frame: &Frame, flipped: bool, progress: u16) -> Vec<u8> {
    let mut px = vec![0u8; FRAME_W * FRAME_H * 4];
    for wire in 0..63u8 {
        let (x, y) = origin(display_cell(wire, flipped));
        paint_terrain(&mut px, x, y, wire);
    }
    let progress = progress.min(1000);
    let anim = progress < 1000 && frame.last_src.is_some() && frame.last_dst.is_some();
    let src = frame.last_src.unwrap_or(0);
    let dst = frame.last_dst.unwrap_or(0);
    let mover = if anim { frame.cells[dst as usize] } else { None };
    if anim {
        if let Some(cap) = frame.captured {
            let (x, y) = origin(display_cell(dst, flipped));
            let fade = ((1000 - u32::from(progress)) * 255 / 1000) as u8;
            blit_piece(&mut px, x, y, cap, fade);
        }
    }
    if let (Some(a), Some(b)) = (frame.last_src, frame.last_dst) {
        mark(&mut px, display_cell(a, flipped));
        mark(&mut px, display_cell(b, flipped));
    }
    for wire in 0..63u8 {
        let Some(piece) = frame.cells[wire as usize] else { continue };
        if anim && wire == dst {
            continue;
        }
        let (x, y) = origin(display_cell(wire, flipped));
        blit_piece(&mut px, x, y, piece, 255);
    }
    if let Some(piece) = mover {
        let (x0, y0) = origin(display_cell(src, flipped));
        let (x1, y1) = origin(display_cell(dst, flipped));
        let t = i32::from(progress);
        let x = x0 + (x1 - x0) * t / 1000;
        let y = y0 + (y1 - y0) * t / 1000;
        blit_piece(&mut px, x, y, piece, 255);
    }
    if frame.phase != "playing" && !frame.phase.is_empty() {
        banner(&mut px, frame);
    }
    for chunk in px.chunks_exact_mut(4) {
        chunk[3] = 255;
    }
    px
}

fn display_cell(wire: u8, flipped: bool) -> u8 {
    if flipped { 62 - wire } else { wire }
}

fn origin(cell: u8) -> (i32, i32) {
    let col = i32::from(cell % 7);
    let row = i32::from(cell / 7);
    (col * CELL as i32, row * CELL as i32)
}

fn paint_terrain(px: &mut [u8], x: i32, y: i32, wire: u8) {
    let col = wire % 7;
    let row = wire / 7;
    let (r, g, b) = if is_water(wire) {
        (23, 138, 148)
    } else if is_den(wire) {
        (12, 8, 6)
    } else if is_trap(wire) {
        (20, 10, 8)
    } else if (col + row) % 2 == 0 {
        (232, 201, 154)
    } else {
        (110, 59, 36)
    };
    fill(px, x, y, CELL as i32, CELL as i32, r, g, b);
    if is_water(wire) {
        for i in 0..3 {
            let yy = y + 18 + i * 14;
            hline(px, x + 10, yy, 44, 220, 255, 255);
        }
    } else if is_trap(wire) {
        line(px, x + 16, y + 16, x + 48, y + 48, 232, 197, 71);
        line(px, x + 48, y + 16, x + 16, y + 48, 232, 197, 71);
    } else if is_den(wire) {
        let (r, g, b) = if wire == 59 { (242, 211, 107) } else { (62, 224, 210) };
        rect(px, x + 4, y + 4, 56, 56, r, g, b);
    }
    rect(px, x, y, CELL as i32, CELL as i32, 30, 14, 8);
}

fn is_water(w: u8) -> bool {
    matches!(w, 22 | 23 | 25 | 26 | 29 | 30 | 32 | 33 | 36 | 37 | 39 | 40)
}
fn is_den(w: u8) -> bool { w == 59 || w == 3 }
fn is_trap(w: u8) -> bool { matches!(w, 52 | 58 | 60 | 2 | 4 | 10) }

fn blit_piece(px: &mut [u8], x: i32, y: i32, piece: SpritePiece, alpha: u8) {
    let idx = piece.color as usize * 8 + piece.kind.saturating_sub(1) as usize;
    if idx >= 16 || piece.kind == 0 {
        return;
    }
    let start = idx * SPRITE * SPRITE * 4;
    let sprite = &RAW[start..start + SPRITE * SPRITE * 4];
    let ox = x + ((CELL - SPRITE) / 2) as i32;
    let oy = y + 2;
    for row in 0..SPRITE {
        for col in 0..SPRITE {
            let s = (row * SPRITE + col) * 4;
            let sa = (u16::from(sprite[s + 3]) * u16::from(alpha) / 255) as u8;
            if sa == 0 {
                continue;
            }
            blend(px, ox + col as i32, oy + row as i32, sprite[s], sprite[s + 1], sprite[s + 2], sa);
        }
    }
    let name = name_of(piece.kind);
    let scale: i32 = if name.len() <= 5 { 2 } else { 1 };
    let width = text_width(name, scale);
    let tx = x + (CELL as i32 - width) / 2;
    let ty = y + CELL as i32 - 8 * scale - 1;
    text(px, tx + 1, ty + 1, name, scale, 12, 8, 6);
    text(px, tx, ty, name, scale, 255, 246, 220);
}

fn banner(px: &mut [u8], frame: &Frame) {
    fill(px, 36, 250, 376, 76, 20, 12, 8);
    rect(px, 36, 250, 376, 76, 201, 162, 39);
    let headline = match frame.phase {
        "won" => match frame.winner {
            "red" => "RED WINS",
            "blue" => "BLUE WINS",
            _ => "WINS",
        },
        "forfeited" => match frame.winner {
            "red" => "RED WINS",
            "blue" => "BLUE WINS",
            _ => "FORFEIT",
        },
        _ => "DRAW",
    };
    let sub = reason_label(frame.reason);
    let hw = text_width(headline, 2);
    text(px, (FRAME_W as i32 - hw) / 2, 262, headline, 2, 243, 230, 200);
    let sw = text_width(sub, 1);
    text(px, (FRAME_W as i32 - sw) / 2, 292, sub, 1, 232, 212, 139);
}

fn reason_label(reason: &str) -> &'static str {
    match reason {
        "den" => "DEN",
        "elimination" => "ELIMINATION",
        "immobilization" => "NO MOVES",
        "repetition" => "REPETITION",
        "no-capture" => "100 QUIET MOVES",
        "timeout" => "TIMEOUT",
        "trap" => "TRAP",
        "invalid-move" => "INVALID MOVE",
        _ => "",
    }
}

fn name_of(kind: u8) -> &'static str {
    match kind {
        1 => "RAT",
        2 => "CAT",
        3 => "DOG",
        4 => "WOLF",
        5 => "LEOPARD",
        6 => "TIGER",
        7 => "LION",
        8 => "ELEPHANT",
        _ => "",
    }
}

fn mark(px: &mut [u8], cell: u8) {
    let (x, y) = origin(cell);
    for row in 0..CELL as i32 {
        for col in 0..CELL as i32 {
            blend(px, x + col, y + row, 242, 211, 107, 70);
        }
    }
    rect(px, x + 2, y + 2, CELL as i32 - 4, CELL as i32 - 4, 242, 211, 107);
}

fn fill(px: &mut [u8], x: i32, y: i32, w: i32, h: i32, r: u8, g: u8, b: u8) {
    for row in 0..h {
        for col in 0..w {
            put(px, x + col, y + row, r, g, b);
        }
    }
}

fn rect(px: &mut [u8], x: i32, y: i32, w: i32, h: i32, r: u8, g: u8, b: u8) {
    hline(px, x, y, w, r, g, b);
    hline(px, x, y + h - 1, w, r, g, b);
    for row in 0..h {
        put(px, x, y + row, r, g, b);
        put(px, x + w - 1, y + row, r, g, b);
    }
}

fn hline(px: &mut [u8], x: i32, y: i32, w: i32, r: u8, g: u8, b: u8) {
    for col in 0..w {
        put(px, x + col, y, r, g, b);
    }
}

fn line(px: &mut [u8], x0: i32, y0: i32, x1: i32, y1: i32, r: u8, g: u8, b: u8) {
    let steps = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
    for i in 0..=steps {
        let x = x0 + (x1 - x0) * i / steps;
        let y = y0 + (y1 - y0) * i / steps;
        put(px, x, y, r, g, b);
    }
}

fn blend(px: &mut [u8], x: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
    if x < 0 || y < 0 || x >= FRAME_W as i32 || y >= FRAME_H as i32 {
        return;
    }
    let i = ((y as usize) * FRAME_W + x as usize) * 4;
    let inv = 255 - u16::from(a);
    px[i] = ((u16::from(r) * u16::from(a) + u16::from(px[i]) * inv) / 255) as u8;
    px[i + 1] = ((u16::from(g) * u16::from(a) + u16::from(px[i + 1]) * inv) / 255) as u8;
    px[i + 2] = ((u16::from(b) * u16::from(a) + u16::from(px[i + 2]) * inv) / 255) as u8;
    px[i + 3] = 255;
}

fn put(px: &mut [u8], x: i32, y: i32, r: u8, g: u8, b: u8) {
    if x < 0 || y < 0 || x >= FRAME_W as i32 || y >= FRAME_H as i32 {
        return;
    }
    let i = ((y as usize) * FRAME_W + x as usize) * 4;
    px[i] = r;
    px[i + 1] = g;
    px[i + 2] = b;
    px[i + 3] = 255;
}

fn text_width(text: &str, scale: i32) -> i32 {
    if text.is_empty() {
        return 0;
    }
    text.len() as i32 * (5 * scale + scale) - scale
}

fn text(px: &mut [u8], x: i32, y: i32, text: &str, scale: i32, r: u8, g: u8, b: u8) {
    let mut cx = x;
    for ch in text.bytes() {
        let rows = glyph(ch);
        for (row, bits) in rows.iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << (4 - col)) != 0 {
                    fill(px, cx + col * scale, y + row as i32 * scale, scale, scale, r, g, b);
                }
            }
        }
        cx += 6 * scale;
    }
}

fn glyph(ch: u8) -> [u8; 7] {
    match ch {
        b'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        b'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        b'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        b'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        b'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        b'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        b'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110],
        b'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        b'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        b'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100],
        b'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        b'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        b'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        b'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        b'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        b'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        b'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        b'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        b'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        b'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        b'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        b'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        b'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010],
        b'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        b'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        b'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        b'0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        b'1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        b'2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        b'3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        b'4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        b'5' => [0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110],
        b'6' => [0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        b'7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        b'8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        b'9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110],
        b'-' => [0, 0, 0, 0b01110, 0, 0, 0],
        b' ' => [0, 0, 0, 0, 0, 0, 0],
        _ => [0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Frame {
        let mut frame = Frame::empty();
        frame.cells[42] = Some(SpritePiece { color: 0, kind: 8 });
        frame.cells[0] = Some(SpritePiece { color: 1, kind: 7 });
        frame.last_src = Some(42);
        frame.last_dst = Some(35);
        frame.cells[35] = Some(SpritePiece { color: 0, kind: 8 });
        frame.cells[42] = None;
        frame
    }

    #[test]
    fn frame_is_opaque_and_flip_differs() {
        let frame = sample();
        let a = render(&frame, false, 1000);
        let b = render(&frame, false, 1000);
        let c = render(&frame, true, 1000);
        assert_eq!(a.len(), FRAME_W * FRAME_H * 4);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.chunks_exact(4).all(|p| p[3] == 255));
    }
}
