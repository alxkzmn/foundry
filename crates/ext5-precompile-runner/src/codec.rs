use alloy_primitives::U256;
use anyhow::ensure;
use p3_field::{BasedVectorSpace, PrimeCharacteristicRing, PrimeField32};

use crate::field_types::{OcticBinExtension, QuinticTrinomialExtension, F};

pub const KOALABEAR_MODULUS: u32 = 0x7f00_0001;

pub fn decode_packed_ext5_word(bytes: &[u8]) -> anyhow::Result<QuinticTrinomialExtension> {
    ensure!(bytes.len() == 32, "packed ext5 word must be 32 bytes, got {}", bytes.len());
    let coeffs = decode_packed_ext5_coeffs(bytes)?;
    Ok(QuinticTrinomialExtension::from_basis_coefficients_fn(|i| F::from_u32(coeffs[i])))
}

pub fn decode_packed_ext5_u256(word: U256) -> anyhow::Result<QuinticTrinomialExtension> {
    decode_packed_ext5_word(&word.to_be_bytes::<32>())
}

pub fn decode_packed_ext8_word(bytes: &[u8]) -> anyhow::Result<OcticBinExtension> {
    ensure!(bytes.len() == 32, "packed ext8 word must be 32 bytes, got {}", bytes.len());
    let coeffs = decode_packed_ext8_coeffs(bytes)?;
    Ok(OcticBinExtension::from_basis_coefficients_fn(|i| F::from_u32(coeffs[i])))
}

pub fn decode_packed_ext8_u256(word: U256) -> anyhow::Result<OcticBinExtension> {
    decode_packed_ext8_word(&word.to_be_bytes::<32>())
}

pub fn encode_packed_ext5_word(value: &QuinticTrinomialExtension) -> [u8; 32] {
    let mut out = [0_u8; 32];
    for (i, coeff) in
        <QuinticTrinomialExtension as BasedVectorSpace<F>>::as_basis_coefficients_slice(value)
            .iter()
            .enumerate()
    {
        let offset = i * 4;
        out[offset..offset + 4].copy_from_slice(&coeff.as_canonical_u32().to_be_bytes());
    }
    out
}

pub fn encode_packed_ext5_u256(value: &QuinticTrinomialExtension) -> U256 {
    U256::from_be_bytes(encode_packed_ext5_word(value))
}

pub fn encode_packed_ext8_word(value: &OcticBinExtension) -> [u8; 32] {
    let mut out = [0_u8; 32];
    for (i, coeff) in <OcticBinExtension as BasedVectorSpace<F>>::as_basis_coefficients_slice(value)
        .iter()
        .enumerate()
    {
        let offset = i * 4;
        out[offset..offset + 4].copy_from_slice(&coeff.as_canonical_u32().to_be_bytes());
    }
    out
}

pub fn encode_packed_ext8_u256(value: &OcticBinExtension) -> U256 {
    U256::from_be_bytes(encode_packed_ext8_word(value))
}

pub fn decode_packed_ext5_coeffs(bytes: &[u8]) -> anyhow::Result<[u32; 5]> {
    ensure!(bytes.len() == 32, "packed ext5 word must be 32 bytes, got {}", bytes.len());
    ensure!(bytes[20..32].iter().all(|byte| *byte == 0), "packed ext5 low 96 bits must be zero");
    let mut coeffs = [0_u32; 5];
    for (i, coeff) in coeffs.iter_mut().enumerate() {
        let offset = i * 4;
        *coeff = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        ensure!(
            *coeff < KOALABEAR_MODULUS,
            "coefficient {} out of range: {} >= {}",
            i,
            *coeff,
            KOALABEAR_MODULUS
        );
    }
    Ok(coeffs)
}

pub fn decode_packed_ext8_coeffs(bytes: &[u8]) -> anyhow::Result<[u32; 8]> {
    ensure!(bytes.len() == 32, "packed ext8 word must be 32 bytes, got {}", bytes.len());
    let mut coeffs = [0_u32; 8];
    for (i, coeff) in coeffs.iter_mut().enumerate() {
        let offset = i * 4;
        *coeff = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        ensure!(
            *coeff < KOALABEAR_MODULUS,
            "coefficient {} out of range: {} >= {}",
            i,
            *coeff,
            KOALABEAR_MODULUS
        );
    }
    Ok(coeffs)
}

pub fn encode_hex_word(bytes: &[u8; 32]) -> String {
    format!("0x{}", hex::encode(bytes))
}
