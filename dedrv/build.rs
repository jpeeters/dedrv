use std::{env, fs::File, io::Write, path::PathBuf};

fn main() -> anyhow::Result<()> {
        // Copy the memory layout to the final build destination.
        let out = &PathBuf::from(env::var("OUT_DIR")?);
        File::create(out.join("dedrv.x"))?.write_all(include_bytes!("dedrv.x"))?;

        // Add the build destination as a linker search path.
        println!("cargo:rustc-link-search={}", out.display());
    }

    Ok(())
}
