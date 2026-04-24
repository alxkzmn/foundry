use std::sync::OnceLock;

use alloy_evm::precompiles::{DynPrecompile, PrecompileInput};
use alloy_primitives::Bytes;
use anvil::PrecompileFactory;
use p3_field::PrimeCharacteristicRing;
use revm::precompile::{PrecompileError, PrecompileOutput, PrecompileResult};
use spartan_whir::engine::{OcticBinExtension, F};

use crate::{
    addresses::{
        EXT8_ADD_ADDRESS, EXT8_MUL_ADDRESS, EXT8_MUL_BASE_ADDRESS, EXT8_MUL_BASE_BATCH_ADDRESS,
        EXT8_MUL_BATCH_ADDRESS, EXT8_SQUARE_ADDRESS, EXT8_SQUARE_BATCH_ADDRESS, EXT8_SUB_ADDRESS,
        NOOP_32_TO_32_ADDRESS, NOOP_64_TO_32_ADDRESS, NOOP_BATCH_32_TO_32_ADDRESS,
        NOOP_BATCH_64_TO_32_ADDRESS,
    },
    codec::{decode_packed_ext8_word, encode_packed_ext8_word, KOALABEAR_MODULUS},
    gas_model::LockedGasSchedule,
};

static LOCKED_GAS_SCHEDULE: OnceLock<LockedGasSchedule> = OnceLock::new();

pub fn install_locked_gas_schedule(schedule: LockedGasSchedule) -> anyhow::Result<()> {
    LOCKED_GAS_SCHEDULE
        .set(schedule)
        .map_err(|_| anyhow::anyhow!("locked gas schedule already installed"))
}

fn locked_gas_schedule() -> &'static LockedGasSchedule {
    LOCKED_GAS_SCHEDULE
        .get()
        .expect("locked gas schedule must be installed before spawning the node")
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Ext8PrecompileFactory;

impl PrecompileFactory for Ext8PrecompileFactory {
    fn precompiles(&self) -> Vec<(alloy_primitives::Address, DynPrecompile)> {
        vec![
            (EXT8_MUL_ADDRESS, DynPrecompile::from(ext8_mul_precompile)),
            (EXT8_SQUARE_ADDRESS, DynPrecompile::from(ext8_square_precompile)),
            (EXT8_ADD_ADDRESS, DynPrecompile::from(ext8_add_precompile)),
            (EXT8_SUB_ADDRESS, DynPrecompile::from(ext8_sub_precompile)),
            (EXT8_MUL_BASE_ADDRESS, DynPrecompile::from(ext8_mul_base_precompile)),
            (EXT8_MUL_BATCH_ADDRESS, DynPrecompile::from(ext8_mul_batch_precompile)),
            (EXT8_SQUARE_BATCH_ADDRESS, DynPrecompile::from(ext8_square_batch_precompile)),
            (EXT8_MUL_BASE_BATCH_ADDRESS, DynPrecompile::from(ext8_mul_base_batch_precompile)),
            (NOOP_64_TO_32_ADDRESS, DynPrecompile::from(noop_64_to_32_precompile)),
            (NOOP_32_TO_32_ADDRESS, DynPrecompile::from(noop_32_to_32_precompile)),
            (NOOP_BATCH_64_TO_32_ADDRESS, DynPrecompile::from(noop_batch_64_to_32_precompile)),
            (NOOP_BATCH_32_TO_32_ADDRESS, DynPrecompile::from(noop_batch_32_to_32_precompile)),
        ]
    }
}

fn ext8_mul_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    let schedule = locked_gas_schedule();
    if input.data.len() != 64 {
        return Err(PrecompileError::other_static("EXT8_MUL expects 64 input bytes"));
    }
    let lhs = decode_word(input.data, 0)?;
    let rhs = decode_word(input.data, 32)?;
    let out = encode_packed_ext8_word(&(lhs * rhs));
    Ok(PrecompileOutput {
        bytes: Bytes::copy_from_slice(&out),
        gas_used: schedule.ext8_mul.assigned_base_gas,
        gas_refunded: 0,
        reverted: false,
    })
}

fn ext8_square_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    let schedule = locked_gas_schedule();
    if input.data.len() != 32 {
        return Err(PrecompileError::other_static("EXT8_SQUARE expects 32 input bytes"));
    }
    let lhs = decode_word(input.data, 0)?;
    let out = encode_packed_ext8_word(&(lhs * lhs));
    Ok(PrecompileOutput {
        bytes: Bytes::copy_from_slice(&out),
        gas_used: schedule.ext8_square.assigned_base_gas,
        gas_refunded: 0,
        reverted: false,
    })
}

fn ext8_add_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    let schedule = locked_gas_schedule();
    if input.data.len() != 64 {
        return Err(PrecompileError::other_static("EXT8_ADD expects 64 input bytes"));
    }
    let lhs = decode_word(input.data, 0)?;
    let rhs = decode_word(input.data, 32)?;
    output_word(lhs + rhs, schedule.ext8_add.assigned_base_gas)
}

fn ext8_sub_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    let schedule = locked_gas_schedule();
    if input.data.len() != 64 {
        return Err(PrecompileError::other_static("EXT8_SUB expects 64 input bytes"));
    }
    let lhs = decode_word(input.data, 0)?;
    let rhs = decode_word(input.data, 32)?;
    output_word(lhs - rhs, schedule.ext8_sub.assigned_base_gas)
}

fn ext8_mul_base_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    let schedule = locked_gas_schedule();
    if input.data.len() != 64 {
        return Err(PrecompileError::other_static("EXT8_MUL_BASE expects 64 input bytes"));
    }
    let lhs = decode_word(input.data, 0)?;
    let scalar = decode_scalar(input.data, 32)?;
    output_word(lhs * scalar, schedule.ext8_mul_base.assigned_base_gas)
}

fn ext8_mul_batch_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    if input.data.len() % 64 != 0 {
        return Err(PrecompileError::other_static("EXT8_MUL_BATCH expects 64 bytes per item"));
    }
    let schedule = locked_gas_schedule();
    let mut out = Vec::with_capacity(input.data.len() / 2);
    for offset in (0..input.data.len()).step_by(64) {
        let lhs = decode_word(input.data, offset)?;
        let rhs = decode_word(input.data, offset + 32)?;
        out.extend_from_slice(&encode_packed_ext8_word(&(lhs * rhs)));
    }
    output_bytes(out, schedule.ext8_mul_batch.assigned_base_gas * (input.data.len() / 64) as u64)
}

fn ext8_square_batch_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    if input.data.len() % 32 != 0 {
        return Err(PrecompileError::other_static("EXT8_SQUARE_BATCH expects 32 bytes per item"));
    }
    let schedule = locked_gas_schedule();
    let mut out = Vec::with_capacity(input.data.len());
    for offset in (0..input.data.len()).step_by(32) {
        let lhs = decode_word(input.data, offset)?;
        out.extend_from_slice(&encode_packed_ext8_word(&(lhs * lhs)));
    }
    output_bytes(out, schedule.ext8_square_batch.assigned_base_gas * (input.data.len() / 32) as u64)
}

fn ext8_mul_base_batch_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    if input.data.len() % 64 != 0 {
        return Err(PrecompileError::other_static("EXT8_MUL_BASE_BATCH expects 64 bytes per item"));
    }
    let schedule = locked_gas_schedule();
    let mut out = Vec::with_capacity(input.data.len() / 2);
    for offset in (0..input.data.len()).step_by(64) {
        let lhs = decode_word(input.data, offset)?;
        let scalar = decode_scalar(input.data, offset + 32)?;
        out.extend_from_slice(&encode_packed_ext8_word(&(lhs * scalar)));
    }
    output_bytes(
        out,
        schedule.ext8_mul_base_batch.assigned_base_gas * (input.data.len() / 64) as u64,
    )
}

fn noop_64_to_32_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    let mut out = [0_u8; 32];
    let take = input.data.len().min(32);
    out[..take].copy_from_slice(&input.data[..take]);
    Ok(PrecompileOutput {
        bytes: Bytes::copy_from_slice(&out),
        gas_used: 0,
        gas_refunded: 0,
        reverted: false,
    })
}

fn noop_32_to_32_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    let mut out = [0_u8; 32];
    let take = input.data.len().min(32);
    out[..take].copy_from_slice(&input.data[..take]);
    Ok(PrecompileOutput {
        bytes: Bytes::copy_from_slice(&out),
        gas_used: 0,
        gas_refunded: 0,
        reverted: false,
    })
}

fn noop_batch_64_to_32_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    if input.data.len() % 64 != 0 {
        return Err(PrecompileError::other_static("NOOP_BATCH_64_TO_32 expects 64 bytes per item"));
    }
    let mut out = Vec::with_capacity(input.data.len() / 2);
    for offset in (0..input.data.len()).step_by(64) {
        out.extend_from_slice(&input.data[offset..offset + 32]);
    }
    output_bytes(out, 0)
}

fn noop_batch_32_to_32_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    if input.data.len() % 32 != 0 {
        return Err(PrecompileError::other_static("NOOP_BATCH_32_TO_32 expects 32 bytes per item"));
    }
    output_bytes(input.data.to_vec(), 0)
}

fn decode_word(input: &[u8], offset: usize) -> Result<OcticBinExtension, PrecompileError> {
    let mut word = [0_u8; 32];
    if input.len() > offset {
        let available = (input.len() - offset).min(32);
        word[..available].copy_from_slice(&input[offset..offset + available]);
    }
    decode_packed_ext8_word(&word)
        .map_err(|err| PrecompileError::Other(format!("invalid packed ext8 input: {err}").into()))
}

fn decode_scalar(input: &[u8], offset: usize) -> Result<F, PrecompileError> {
    if input.len() < offset + 32 {
        return Err(PrecompileError::other_static("scalar input is truncated"));
    }
    if input[offset..offset + 28].iter().any(|byte| *byte != 0) {
        return Err(PrecompileError::other_static("base scalar high bytes must be zero"));
    }
    let value = u32::from_be_bytes(input[offset + 28..offset + 32].try_into().unwrap());
    if value >= KOALABEAR_MODULUS {
        return Err(PrecompileError::other_static("base scalar out of range"));
    }
    Ok(F::from_u32(value))
}

fn output_word(value: OcticBinExtension, gas_used: u64) -> PrecompileResult {
    let out = encode_packed_ext8_word(&value);
    output_bytes(out.to_vec(), gas_used)
}

fn output_bytes(bytes: Vec<u8>, gas_used: u64) -> PrecompileResult {
    Ok(PrecompileOutput { bytes: Bytes::from(bytes), gas_used, gas_refunded: 0, reverted: false })
}
