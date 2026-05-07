use std::sync::OnceLock;

use alloy_evm::precompiles::{DynPrecompile, PrecompileInput};
use alloy_primitives::Bytes;
use anvil::PrecompileFactory;
use p3_field::PrimeCharacteristicRing;
use revm::precompile::{PrecompileError, PrecompileOutput, PrecompileResult};

use crate::{
    addresses::{
        EXT5_ADD_ADDRESS, EXT5_MUL_ADDRESS, EXT5_MUL_BASE_ADDRESS, EXT5_MUL_BASE_BATCH_ADDRESS,
        EXT5_MUL_BATCH_ADDRESS, EXT5_SQUARE_ADDRESS, EXT5_SQUARE_BATCH_ADDRESS, EXT5_SUB_ADDRESS,
        EXTFIELD_LIN_PROD_ADDRESS, EXTFIELD_MAC_ADDRESS, EXTFIELD_MAC_FIELD_ID_KOALABEAR_EXT5,
        EXTFIELD_MAC_FIELD_ID_KOALABEAR_EXT8, NOOP_32_TO_32_ADDRESS, NOOP_64_TO_32_ADDRESS,
        NOOP_BATCH_32_TO_32_ADDRESS, NOOP_BATCH_64_TO_32_ADDRESS, NOOP_EXTFIELD_LIN_PROD_ADDRESS,
        NOOP_EXTFIELD_MAC_ADDRESS,
    },
    codec::{
        decode_packed_ext5_word, decode_packed_ext8_word, encode_packed_ext5_word,
        encode_packed_ext8_word, KOALABEAR_MODULUS,
    },
    field_types::{OcticBinExtension, QuinticTrinomialExtension, F},
    gas_model::{
        ExtfieldLinProdFieldGasSchedule, ExtfieldLinProdGasModel, ExtfieldLinProdGasSchedule,
        ExtfieldMacFieldGasSchedule, ExtfieldMacGasSchedule, LockedGasSchedule,
        EXTFIELD_LIN_PROD_FLAG_ALPHA_ONE_BASE_BETA, EXTFIELD_LIN_PROD_FLAG_ALPHA_ONE_EXT_BETA,
        EXTFIELD_LIN_PROD_FLAG_EXPLICIT,
    },
};

static LOCKED_GAS_SCHEDULE: OnceLock<LockedGasSchedule> = OnceLock::new();
static LOCKED_MAC_GAS_SCHEDULE: OnceLock<ExtfieldMacGasSchedule> = OnceLock::new();
static LOCKED_LIN_PROD_GAS_SCHEDULE: OnceLock<ExtfieldLinProdGasSchedule> = OnceLock::new();

pub fn install_locked_gas_schedule(schedule: LockedGasSchedule) -> anyhow::Result<()> {
    LOCKED_GAS_SCHEDULE
        .set(schedule)
        .map_err(|_| anyhow::anyhow!("locked gas schedule already installed"))
}

pub fn install_locked_mac_gas_schedule(schedule: ExtfieldMacGasSchedule) -> anyhow::Result<()> {
    LOCKED_MAC_GAS_SCHEDULE
        .set(schedule)
        .map_err(|_| anyhow::anyhow!("locked MAC gas schedule already installed"))
}

pub fn install_locked_lin_prod_gas_schedule(
    schedule: ExtfieldLinProdGasSchedule,
) -> anyhow::Result<()> {
    LOCKED_LIN_PROD_GAS_SCHEDULE
        .set(schedule)
        .map_err(|_| anyhow::anyhow!("locked LIN_PROD gas schedule already installed"))
}

fn locked_gas_schedule() -> &'static LockedGasSchedule {
    LOCKED_GAS_SCHEDULE
        .get()
        .expect("locked gas schedule must be installed before spawning the node")
}

fn locked_mac_gas_schedule() -> &'static ExtfieldMacGasSchedule {
    LOCKED_MAC_GAS_SCHEDULE
        .get()
        .expect("locked MAC gas schedule must be installed before spawning the node")
}

fn locked_lin_prod_gas_schedule() -> &'static ExtfieldLinProdGasSchedule {
    LOCKED_LIN_PROD_GAS_SCHEDULE
        .get()
        .expect("locked LIN_PROD gas schedule must be installed before spawning the node")
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Ext5PrecompileFactory;

impl PrecompileFactory for Ext5PrecompileFactory {
    fn precompiles(&self) -> Vec<(alloy_primitives::Address, DynPrecompile)> {
        vec![
            (EXT5_MUL_ADDRESS, DynPrecompile::from(ext5_mul_precompile)),
            (EXT5_SQUARE_ADDRESS, DynPrecompile::from(ext5_square_precompile)),
            (EXT5_ADD_ADDRESS, DynPrecompile::from(ext5_add_precompile)),
            (EXT5_SUB_ADDRESS, DynPrecompile::from(ext5_sub_precompile)),
            (EXT5_MUL_BASE_ADDRESS, DynPrecompile::from(ext5_mul_base_precompile)),
            (EXT5_MUL_BATCH_ADDRESS, DynPrecompile::from(ext5_mul_batch_precompile)),
            (EXT5_SQUARE_BATCH_ADDRESS, DynPrecompile::from(ext5_square_batch_precompile)),
            (EXT5_MUL_BASE_BATCH_ADDRESS, DynPrecompile::from(ext5_mul_base_batch_precompile)),
            (EXTFIELD_MAC_ADDRESS, DynPrecompile::from(extfield_mac_precompile)),
            (EXTFIELD_LIN_PROD_ADDRESS, DynPrecompile::from(extfield_lin_prod_precompile)),
            (NOOP_64_TO_32_ADDRESS, DynPrecompile::from(noop_64_to_32_precompile)),
            (NOOP_32_TO_32_ADDRESS, DynPrecompile::from(noop_32_to_32_precompile)),
            (NOOP_BATCH_64_TO_32_ADDRESS, DynPrecompile::from(noop_batch_64_to_32_precompile)),
            (NOOP_BATCH_32_TO_32_ADDRESS, DynPrecompile::from(noop_batch_32_to_32_precompile)),
            (NOOP_EXTFIELD_MAC_ADDRESS, DynPrecompile::from(noop_extfield_mac_precompile)),
            (
                NOOP_EXTFIELD_LIN_PROD_ADDRESS,
                DynPrecompile::from(noop_extfield_lin_prod_precompile),
            ),
        ]
    }
}

fn ext5_mul_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    let schedule = locked_gas_schedule();
    if input.data.len() != 64 {
        return Err(PrecompileError::other_static("EXT5_MUL expects 64 input bytes"));
    }
    let lhs = decode_word(input.data, 0)?;
    let rhs = decode_word(input.data, 32)?;
    let out = encode_packed_ext5_word(&(lhs * rhs));
    Ok(PrecompileOutput {
        bytes: Bytes::copy_from_slice(&out),
        gas_used: schedule.ext5_mul.assigned_base_gas,
        gas_refunded: 0,
        reverted: false,
    })
}

fn ext5_square_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    let schedule = locked_gas_schedule();
    if input.data.len() != 32 {
        return Err(PrecompileError::other_static("EXT5_SQUARE expects 32 input bytes"));
    }
    let lhs = decode_word(input.data, 0)?;
    let out = encode_packed_ext5_word(&(lhs * lhs));
    Ok(PrecompileOutput {
        bytes: Bytes::copy_from_slice(&out),
        gas_used: schedule.ext5_square.assigned_base_gas,
        gas_refunded: 0,
        reverted: false,
    })
}

fn ext5_add_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    let schedule = locked_gas_schedule();
    if input.data.len() != 64 {
        return Err(PrecompileError::other_static("EXT5_ADD expects 64 input bytes"));
    }
    let lhs = decode_word(input.data, 0)?;
    let rhs = decode_word(input.data, 32)?;
    output_word(lhs + rhs, schedule.ext5_add.assigned_base_gas)
}

fn ext5_sub_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    let schedule = locked_gas_schedule();
    if input.data.len() != 64 {
        return Err(PrecompileError::other_static("EXT5_SUB expects 64 input bytes"));
    }
    let lhs = decode_word(input.data, 0)?;
    let rhs = decode_word(input.data, 32)?;
    output_word(lhs - rhs, schedule.ext5_sub.assigned_base_gas)
}

fn ext5_mul_base_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    let schedule = locked_gas_schedule();
    if input.data.len() != 64 {
        return Err(PrecompileError::other_static("EXT5_MUL_BASE expects 64 input bytes"));
    }
    let lhs = decode_word(input.data, 0)?;
    let scalar = decode_scalar(input.data, 32)?;
    output_word(lhs * scalar, schedule.ext5_mul_base.assigned_base_gas)
}

fn ext5_mul_batch_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    if input.data.len() % 64 != 0 {
        return Err(PrecompileError::other_static("EXT5_MUL_BATCH expects 64 bytes per item"));
    }
    let schedule = locked_gas_schedule();
    let mut out = Vec::with_capacity(input.data.len() / 2);
    for offset in (0..input.data.len()).step_by(64) {
        let lhs = decode_word(input.data, offset)?;
        let rhs = decode_word(input.data, offset + 32)?;
        out.extend_from_slice(&encode_packed_ext5_word(&(lhs * rhs)));
    }
    output_bytes(out, schedule.ext5_mul_batch.assigned_base_gas * (input.data.len() / 64) as u64)
}

fn ext5_square_batch_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    if input.data.len() % 32 != 0 {
        return Err(PrecompileError::other_static("EXT5_SQUARE_BATCH expects 32 bytes per item"));
    }
    let schedule = locked_gas_schedule();
    let mut out = Vec::with_capacity(input.data.len());
    for offset in (0..input.data.len()).step_by(32) {
        let lhs = decode_word(input.data, offset)?;
        out.extend_from_slice(&encode_packed_ext5_word(&(lhs * lhs)));
    }
    output_bytes(out, schedule.ext5_square_batch.assigned_base_gas * (input.data.len() / 32) as u64)
}

fn ext5_mul_base_batch_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    if input.data.len() % 64 != 0 {
        return Err(PrecompileError::other_static("EXT5_MUL_BASE_BATCH expects 64 bytes per item"));
    }
    let schedule = locked_gas_schedule();
    let mut out = Vec::with_capacity(input.data.len() / 2);
    for offset in (0..input.data.len()).step_by(64) {
        let lhs = decode_word(input.data, offset)?;
        let scalar = decode_scalar(input.data, offset + 32)?;
        out.extend_from_slice(&encode_packed_ext5_word(&(lhs * scalar)));
    }
    output_bytes(
        out,
        schedule.ext5_mul_base_batch.assigned_base_gas * (input.data.len() / 64) as u64,
    )
}

fn extfield_mac_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    let schedule = locked_mac_gas_schedule();
    let request = parse_mac_header(input.data, schedule)?;
    let gas_used = request.field.extfield_mac.assigned_base_gas
        + request.field.extfield_mac.assigned_per_pair_gas * request.n as u64;
    match request.field.field_id {
        EXTFIELD_MAC_FIELD_ID_KOALABEAR_EXT5 => {
            output_ext5_word(mac_ext5(input.data, request)?, gas_used)
        }
        EXTFIELD_MAC_FIELD_ID_KOALABEAR_EXT8 => {
            output_ext8_word(mac_ext8(input.data, request)?, gas_used)
        }
        _ => Err(PrecompileError::other_static("EXTFIELD_MAC unsupported field_id")),
    }
}

fn extfield_lin_prod_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    let schedule = locked_lin_prod_gas_schedule();
    let request = parse_lin_prod_header(input.data, schedule)?;
    let gas_used =
        request.mode.assigned_base_gas + request.mode.assigned_per_term_gas * request.n as u64;
    match request.field.field_id {
        EXTFIELD_MAC_FIELD_ID_KOALABEAR_EXT5 => {
            output_ext5_word(lin_prod_ext5(input.data, request)?, gas_used)
        }
        EXTFIELD_MAC_FIELD_ID_KOALABEAR_EXT8 => {
            output_ext8_word(lin_prod_ext8(input.data, request)?, gas_used)
        }
        _ => Err(PrecompileError::other_static("EXTFIELD_LIN_PROD unsupported field_id")),
    }
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

fn noop_extfield_mac_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    let schedule = locked_mac_gas_schedule();
    let _ = parse_mac_header(input.data, schedule)?;
    output_bytes(vec![0_u8; 32], 0)
}

fn noop_extfield_lin_prod_precompile(input: PrecompileInput<'_>) -> PrecompileResult {
    let schedule = locked_lin_prod_gas_schedule();
    let _ = parse_lin_prod_header(input.data, schedule)?;
    output_bytes(vec![0_u8; 32], 0)
}

#[derive(Debug, Clone, Copy)]
struct MacRequest<'a> {
    n: usize,
    has_accumulator: bool,
    field: &'a ExtfieldMacFieldGasSchedule,
}

#[derive(Debug, Clone, Copy)]
struct LinProdRequest<'a> {
    n: usize,
    flags: u32,
    field: &'a ExtfieldLinProdFieldGasSchedule,
    mode: &'a ExtfieldLinProdGasModel,
}

fn parse_mac_header<'a>(
    input: &[u8],
    schedule: &'a ExtfieldMacGasSchedule,
) -> Result<MacRequest<'a>, PrecompileError> {
    if input.len() < 8 {
        return Err(PrecompileError::other_static("EXTFIELD_MAC input is shorter than header"));
    }
    let field_id = u16::from_be_bytes(input[0..2].try_into().unwrap());
    let Some(field) = schedule.field(field_id) else {
        return Err(PrecompileError::other_static("EXTFIELD_MAC unknown field_id"));
    };
    let n = u16::from_be_bytes(input[2..4].try_into().unwrap()) as usize;
    if n > field.n_max {
        return Err(PrecompileError::other_static("EXTFIELD_MAC n exceeds configured maximum"));
    }
    let flags = u32::from_be_bytes(input[4..8].try_into().unwrap());
    if flags & !1 != 0 {
        return Err(PrecompileError::other_static("EXTFIELD_MAC reserved flag bit set"));
    }
    let has_accumulator = flags & 1 != 0;
    let expected_len = 8 + if has_accumulator { 32 } else { 0 } + 64 * n;
    if input.len() != expected_len {
        return Err(PrecompileError::other_static("EXTFIELD_MAC input length mismatch"));
    }
    Ok(MacRequest { n, has_accumulator, field })
}

fn mac_ext5(
    input: &[u8],
    request: MacRequest<'_>,
) -> Result<QuinticTrinomialExtension, PrecompileError> {
    let mut offset = 8usize;
    let mut acc = QuinticTrinomialExtension::ZERO;
    if request.has_accumulator {
        acc = decode_word(input, offset)?;
        offset += 32;
    }
    for _ in 0..request.n {
        let lhs = decode_word(input, offset)?;
        let rhs = decode_word(input, offset + 32)?;
        acc += lhs * rhs;
        offset += 64;
    }
    Ok(acc)
}

fn mac_ext8(input: &[u8], request: MacRequest<'_>) -> Result<OcticBinExtension, PrecompileError> {
    let mut offset = 8usize;
    let mut acc = OcticBinExtension::ZERO;
    if request.has_accumulator {
        acc = decode_ext8_word(input, offset)?;
        offset += 32;
    }
    for _ in 0..request.n {
        let lhs = decode_ext8_word(input, offset)?;
        let rhs = decode_ext8_word(input, offset + 32)?;
        acc += lhs * rhs;
        offset += 64;
    }
    Ok(acc)
}

fn parse_lin_prod_header<'a>(
    input: &[u8],
    schedule: &'a ExtfieldLinProdGasSchedule,
) -> Result<LinProdRequest<'a>, PrecompileError> {
    if input.len() < 8 {
        return Err(PrecompileError::other_static(
            "EXTFIELD_LIN_PROD input is shorter than header",
        ));
    }
    let field_id = u16::from_be_bytes(input[0..2].try_into().unwrap());
    let Some((field, mode)) =
        schedule.mode(field_id, u32::from_be_bytes(input[4..8].try_into().unwrap()))
    else {
        return Err(PrecompileError::other_static("EXTFIELD_LIN_PROD unknown field_id or flags"));
    };
    let n = u16::from_be_bytes(input[2..4].try_into().unwrap()) as usize;
    if n > field.n_max {
        return Err(PrecompileError::other_static(
            "EXTFIELD_LIN_PROD n exceeds configured maximum",
        ));
    }
    let flags = mode.flags;
    let bytes_per_term = match flags {
        EXTFIELD_LIN_PROD_FLAG_EXPLICIT => 96,
        EXTFIELD_LIN_PROD_FLAG_ALPHA_ONE_EXT_BETA => 64,
        EXTFIELD_LIN_PROD_FLAG_ALPHA_ONE_BASE_BETA => 36,
        _ => {
            return Err(PrecompileError::other_static("EXTFIELD_LIN_PROD reserved flag bit set"));
        }
    };
    let expected_len = 8 + bytes_per_term * n;
    if input.len() != expected_len {
        return Err(PrecompileError::other_static("EXTFIELD_LIN_PROD input length mismatch"));
    }
    Ok(LinProdRequest { n, flags, field, mode })
}

fn lin_prod_ext5(
    input: &[u8],
    request: LinProdRequest<'_>,
) -> Result<QuinticTrinomialExtension, PrecompileError> {
    let mut offset = 8usize;
    let mut acc = QuinticTrinomialExtension::ONE;
    for _ in 0..request.n {
        let term = match request.flags {
            EXTFIELD_LIN_PROD_FLAG_EXPLICIT => {
                let alpha = decode_word(input, offset)?;
                let beta = decode_word(input, offset + 32)?;
                let x = decode_word(input, offset + 64)?;
                offset += 96;
                alpha + beta * x
            }
            EXTFIELD_LIN_PROD_FLAG_ALPHA_ONE_EXT_BETA => {
                let beta = decode_word(input, offset)?;
                let x = decode_word(input, offset + 32)?;
                offset += 64;
                QuinticTrinomialExtension::ONE + beta * x
            }
            EXTFIELD_LIN_PROD_FLAG_ALPHA_ONE_BASE_BETA => {
                let beta = decode_scalar4(input, offset)?;
                let x = decode_word(input, offset + 4)?;
                offset += 36;
                QuinticTrinomialExtension::ONE + x * beta
            }
            _ => unreachable!("unsupported LIN_PROD flags"),
        };
        acc *= term;
    }
    Ok(acc)
}

fn lin_prod_ext8(
    input: &[u8],
    request: LinProdRequest<'_>,
) -> Result<OcticBinExtension, PrecompileError> {
    let mut offset = 8usize;
    let mut acc = OcticBinExtension::ONE;
    for _ in 0..request.n {
        let term = match request.flags {
            EXTFIELD_LIN_PROD_FLAG_EXPLICIT => {
                let alpha = decode_ext8_word(input, offset)?;
                let beta = decode_ext8_word(input, offset + 32)?;
                let x = decode_ext8_word(input, offset + 64)?;
                offset += 96;
                alpha + beta * x
            }
            EXTFIELD_LIN_PROD_FLAG_ALPHA_ONE_EXT_BETA => {
                let beta = decode_ext8_word(input, offset)?;
                let x = decode_ext8_word(input, offset + 32)?;
                offset += 64;
                OcticBinExtension::ONE + beta * x
            }
            EXTFIELD_LIN_PROD_FLAG_ALPHA_ONE_BASE_BETA => {
                let beta = decode_scalar4(input, offset)?;
                let x = decode_ext8_word(input, offset + 4)?;
                offset += 36;
                OcticBinExtension::ONE + x * beta
            }
            _ => unreachable!("unsupported LIN_PROD flags"),
        };
        acc *= term;
    }
    Ok(acc)
}

fn decode_word(input: &[u8], offset: usize) -> Result<QuinticTrinomialExtension, PrecompileError> {
    let mut word = [0_u8; 32];
    if input.len() > offset {
        let available = (input.len() - offset).min(32);
        word[..available].copy_from_slice(&input[offset..offset + available]);
    }
    decode_packed_ext5_word(&word)
        .map_err(|err| PrecompileError::Other(format!("invalid packed ext5 input: {err}").into()))
}

fn decode_ext8_word(input: &[u8], offset: usize) -> Result<OcticBinExtension, PrecompileError> {
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

fn decode_scalar4(input: &[u8], offset: usize) -> Result<F, PrecompileError> {
    if input.len() < offset + 4 {
        return Err(PrecompileError::other_static("base scalar input is truncated"));
    }
    let value = u32::from_be_bytes(input[offset..offset + 4].try_into().unwrap());
    if value >= KOALABEAR_MODULUS {
        return Err(PrecompileError::other_static("base scalar out of range"));
    }
    Ok(F::from_u32(value))
}

fn output_word(value: QuinticTrinomialExtension, gas_used: u64) -> PrecompileResult {
    let out = encode_packed_ext5_word(&value);
    output_bytes(out.to_vec(), gas_used)
}

fn output_ext5_word(value: QuinticTrinomialExtension, gas_used: u64) -> PrecompileResult {
    output_word(value, gas_used)
}

fn output_ext8_word(value: OcticBinExtension, gas_used: u64) -> PrecompileResult {
    let out = encode_packed_ext8_word(&value);
    output_bytes(out.to_vec(), gas_used)
}

fn output_bytes(bytes: Vec<u8>, gas_used: u64) -> PrecompileResult {
    Ok(PrecompileOutput { bytes: Bytes::from(bytes), gas_used, gas_refunded: 0, reverted: false })
}
