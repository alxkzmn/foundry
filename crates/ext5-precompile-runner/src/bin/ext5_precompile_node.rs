use std::{env, path::PathBuf};

use anvil::NodeConfig;
use ext5_precompile_runner::{
    gas_model::{ExtfieldMacGasSchedule, LockedGasSchedule},
    precompiles::{
        install_locked_gas_schedule, install_locked_mac_gas_schedule, Ext5PrecompileFactory,
    },
};

#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() -> anyhow::Result<()> {
    let mut args = env::args().skip(1);
    let gas_schedule_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from("sol-spartan-whir/testdata/ext5_precompile_gas_schedule.json")
    });
    let mut mac_gas_schedule_path =
        PathBuf::from("sol-spartan-whir/testdata/extfield_mac_gas_schedule.json");
    let mut port = 18547;
    if let Some(second) = args.next() {
        match second.parse::<u16>() {
            Ok(parsed_port) => {
                port = parsed_port;
            }
            Err(_) => {
                mac_gas_schedule_path = PathBuf::from(second);
                if let Some(third) = args.next() {
                    port = third.parse::<u16>()?;
                }
            }
        }
    }

    let schedule = LockedGasSchedule::read_json(&gas_schedule_path)?;
    let mac_schedule = ExtfieldMacGasSchedule::read_json(&mac_gas_schedule_path)?;
    install_locked_gas_schedule(schedule.clone())?;
    install_locked_mac_gas_schedule(mac_schedule.clone())?;

    println!("starting custom anvil node with ext5 precompiles");
    println!("gas schedule: {}", gas_schedule_path.display());
    println!("MAC gas schedule: {}", mac_gas_schedule_path.display());
    println!("port: {}", port);
    println!("ext5_mul assigned base gas: {}", schedule.ext5_mul.assigned_base_gas);
    println!("ext5_square assigned base gas: {}", schedule.ext5_square.assigned_base_gas);
    for field in &mac_schedule.fields {
        println!(
            "extfield_mac field_id={} assigned gas: base={} per_pair={}",
            field.field_id,
            field.extfield_mac.assigned_base_gas,
            field.extfield_mac.assigned_per_pair_gas
        );
    }

    let config = NodeConfig::default()
        .silent()
        .with_port(port)
        .with_code_size_limit(Some(50_000))
        .with_precompile_factory(Ext5PrecompileFactory);
    let (_api, handle) = anvil::spawn(config).await;
    println!("http endpoint: {}", handle.http_endpoint());
    handle.await??;
    Ok(())
}
