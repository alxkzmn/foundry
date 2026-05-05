use std::{env, path::PathBuf};

use anyhow::Context;
use ext5_precompile_runner::vectors::{write_mac_vector_outputs, DEFAULT_MAC_VECTOR_REPEATS};

fn main() -> anyhow::Result<()> {
    let out_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("sol-spartan-whir/testdata"));
    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("failed to create {}", out_dir.display()))?;
    write_mac_vector_outputs(&out_dir, DEFAULT_MAC_VECTOR_REPEATS)?;
    println!("wrote EXTFIELD_MAC vectors to {}", out_dir.display());
    Ok(())
}
