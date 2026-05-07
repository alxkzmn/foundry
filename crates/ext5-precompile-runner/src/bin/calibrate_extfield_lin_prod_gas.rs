use std::{env, path::PathBuf};

use anyhow::Context;
use ext5_precompile_runner::gas_model::ExtfieldLinProdGasSchedule;

fn main() -> anyhow::Result<()> {
    let out_path = env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("sol-spartan-whir/testdata/extfield_lin_prod_gas_schedule.json")
    });

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let schedule = ExtfieldLinProdGasSchedule::from_live_benchmark();
    schedule.write_json(&out_path)?;

    println!("wrote {}", out_path.display());
    for field in &schedule.fields {
        println!("field_id: {}", field.field_id);
        println!("n_max: {}", field.n_max);
        for mode in &field.modes {
            println!(
                "lin_prod flags={} assigned gas: base={} per_term={}",
                mode.flags, mode.assigned_base_gas, mode.assigned_per_term_gas
            );
            for sample in &mode.samples {
                println!(
                    "  n={} median runtime (ns): {} assigned gas at n: {}",
                    sample.n, sample.median_runtime_ns, sample.assigned_gas_at_n
                );
            }
            for sample in &mode.measurement_samples {
                println!(
                    "  measurement n={} median runtime (ns): {} assigned gas at n: {}",
                    sample.n, sample.median_runtime_ns, sample.assigned_gas_at_n
                );
            }
        }
    }
    Ok(())
}
