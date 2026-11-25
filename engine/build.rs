use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let pieces = manifest.join("../public/assets/pieces");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut raw = Vec::new();
    for side in ["gold", "scarlet"] {
        for animal in ["rat", "cat", "dog", "wolf", "leopard", "tiger", "lion", "elephant"] {
            let path = pieces.join(side).join(format!("{animal}.png"));
            println!("cargo:rerun-if-changed={}", path.display());
            let img = image::open(&path)
                .unwrap_or_else(|err| panic!("open {}: {err}", path.display()))
                .resize_exact(52, 52, image::imageops::FilterType::Triangle)
                .to_rgba8();
            raw.extend_from_slice(img.as_raw());
        }
    }
    fs::write(out_dir.join("sprites.bin"), &raw).unwrap();
}
