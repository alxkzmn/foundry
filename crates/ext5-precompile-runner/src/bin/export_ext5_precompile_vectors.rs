use std::{env, path::PathBuf};

use anyhow::Context;
use ext5_precompile_runner::vectors::{write_vector_outputs, DEFAULT_VECTOR_COUNT};

fn main() -> anyhow::Result<()> {
    let out_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("sol-spartan-whir/testdata"));
    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("failed to create {}", out_dir.display()))?;
    write_vector_outputs(&out_dir, DEFAULT_VECTOR_COUNT)?;
    println!("wrote ext5 precompile vectors to {}", out_dir.display());
    Ok(())
}
