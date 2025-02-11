use std::{env, fs::File, io::Write, path::PathBuf};

fn main() -> anyhow::Result<()> {
    let out = &PathBuf::from(env::var("OUT_DIR")?);
    File::create(out.join("memory.x"))?.write_all(include_bytes!("memory.x"))?;

    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory.x");

    // Linker args used by `cortex-m-rt` crate.
    println!("cargo:rustc-link-arg=-nmagic");
    println!("cargo:rustc-link-arg=-Tlink.x");

    // Linker args used by `defmt` crate.
    println!("cargo:rustc-link-arg=-Tdefmt.x");

    // Linker args used by `dedrv` crate.
    println!("cargo:rustc-link-arg=-Tdedrv.x");

    Ok(())
}
