use std::{env, path::PathBuf};

use anyhow::Context;
use ext8_precompile_runner::gas_model::LockedGasSchedule;

fn main() -> anyhow::Result<()> {
    let out_path = env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("sol-spartan-whir/testdata/ext8_precompile_gas_schedule.json")
    });

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let schedule = LockedGasSchedule::from_live_benchmark();
    schedule.write_json(&out_path)?;

    println!("wrote {}", out_path.display());
    println!("ext8_mul median runtime (ns): {}", schedule.ext8_mul.median_runtime_ns);
    println!("ext8_mul assigned base gas: {}", schedule.ext8_mul.assigned_base_gas);
    println!("ext8_square median runtime (ns): {}", schedule.ext8_square.median_runtime_ns);
    println!("ext8_square assigned base gas: {}", schedule.ext8_square.assigned_base_gas);
    Ok(())
}
