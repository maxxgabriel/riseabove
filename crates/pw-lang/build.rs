//! Embeds every `data/lang/<locale>/**/*.toml` file so the language data ships inside the binary without a hand-kept list.
use std::fmt::Write;
use std::path::{Path, PathBuf};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "toml") {
            out.push(p);
        }
    }
}

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../data/lang");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    walk(&root, &mut files);
    files.sort();
    let mut src = String::from("pub static FILES: &[(&str, &str)] = &[\n");
    for f in &files {
        let rel = f.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
        writeln!(src, "    ({rel:?}, include_str!({:?})),", f.canonicalize().unwrap().to_string_lossy()).unwrap();
        println!("cargo:rerun-if-changed={}", f.display());
    }
    src.push_str("];\n");
    std::fs::write(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("embedded.rs"), src).unwrap();
}
