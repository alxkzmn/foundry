use std::{env, path::PathBuf};

use anyhow::Context;
use ext5_precompile_runner::gas_model::LockedGasSchedule;

fn main() -> anyhow::Result<()> {
    let out_path = env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("sol-spartan-whir/testdata/ext5_precompile_gas_schedule.json")
    });

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let schedule = LockedGasSchedule::from_live_benchmark();
    schedule.write_json(&out_path)?;

    println!("wrote {}", out_path.display());
    println!("ext5_mul median runtime (ns): {}", schedule.ext5_mul.median_runtime_ns);
    println!("ext5_mul assigned base gas: {}", schedule.ext5_mul.assigned_base_gas);
    println!("ext5_square median runtime (ns): {}", schedule.ext5_square.median_runtime_ns);
    println!("ext5_square assigned base gas: {}", schedule.ext5_square.assigned_base_gas);
    Ok(())
}
