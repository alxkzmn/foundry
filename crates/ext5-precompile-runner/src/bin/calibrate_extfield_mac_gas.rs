use std::{env, path::PathBuf};

use anyhow::Context;
use ext5_precompile_runner::gas_model::ExtfieldMacGasSchedule;

fn main() -> anyhow::Result<()> {
    let out_path = env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("sol-spartan-whir/testdata/extfield_mac_gas_schedule.json")
    });

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let schedule = ExtfieldMacGasSchedule::from_live_benchmark();
    schedule.write_json(&out_path)?;

    println!("wrote {}", out_path.display());
    println!("field_id: {}", schedule.field_id);
    println!("n_max: {}", schedule.n_max);
    println!(
        "extfield_mac assigned gas: base={} per_pair={}",
        schedule.extfield_mac.assigned_base_gas, schedule.extfield_mac.assigned_per_pair_gas
    );
    for sample in &schedule.extfield_mac.samples {
        println!(
            "n={} median runtime (ns): {} assigned gas at n: {}",
            sample.n, sample.median_runtime_ns, sample.assigned_gas_at_n
        );
    }
    Ok(())
}
