use std::{env, path::PathBuf};

use anvil::NodeConfig;
use ext8_precompile_runner::{
    gas_model::LockedGasSchedule,
    precompiles::{install_locked_gas_schedule, Ext8PrecompileFactory},
};

#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() -> anyhow::Result<()> {
    let mut args = env::args().skip(1);
    let gas_schedule_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("sol-spartan-whir/testdata/ext8_precompile_gas_schedule.json")
    });
    let port = args.next().map(|raw| raw.parse::<u16>()).transpose()?.unwrap_or(18547);

    let schedule = LockedGasSchedule::read_json(&gas_schedule_path)?;
    install_locked_gas_schedule(schedule.clone())?;

    println!("starting custom anvil node with ext8 precompiles");
    println!("gas schedule: {}", gas_schedule_path.display());
    println!("port: {}", port);
    println!("ext8_mul assigned base gas: {}", schedule.ext8_mul.assigned_base_gas);
    println!("ext8_square assigned base gas: {}", schedule.ext8_square.assigned_base_gas);

    let config = NodeConfig::default()
        .silent()
        .with_port(port)
        .with_code_size_limit(Some(50_000))
        .with_precompile_factory(Ext8PrecompileFactory);
    let (_api, handle) = anvil::spawn(config).await;
    println!("http endpoint: {}", handle.http_endpoint());
    handle.await??;
    Ok(())
}
