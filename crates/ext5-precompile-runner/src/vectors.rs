use std::{fs, path::Path};

use alloy_primitives::U256;
use alloy_sol_types::{sol, SolValue};
use anyhow::Context;
use p3_field::{BasedVectorSpace, PrimeCharacteristicRing, PrimeField32};
use serde::{Deserialize, Serialize};

use crate::{
    codec::{encode_hex_word, encode_packed_ext5_word},
    field_types::{QuinticTrinomialExtension, F},
};

pub const VECTOR_SEED: u64 = 0xE8C0_5A71_1234_5678;
pub const DEFAULT_VECTOR_COUNT: usize = 10_000;

#[derive(Debug, Clone, Copy)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        let state = if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed };
        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_base_field(&mut self) -> F {
        F::from_u32((self.next_u64() as u32) % 0x7f00_0001)
    }

    fn next_quintic(&mut self) -> QuinticTrinomialExtension {
        QuinticTrinomialExtension::from_basis_coefficients_fn(|_| self.next_base_field())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ext5ArithmeticVector {
    pub packed_a: String,
    pub packed_b: String,
    pub scalar: String,
    pub packed_add: String,
    pub packed_sub: String,
    pub packed_mul: String,
    pub packed_square_a: String,
    pub packed_mul_base: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ext5ArithmeticVectorFile {
    pub seed: u64,
    pub count: usize,
    pub vectors: Vec<Ext5ArithmeticVector>,
}

sol! {
    struct Ext5ArithmeticVectorsAbi {
        uint256[] packedA;
        uint256[] packedB;
        uint256[] scalars;
        uint256[] packedAdd;
        uint256[] packedSub;
        uint256[] packedMul;
        uint256[] packedSquareA;
        uint256[] packedMulBase;
    }
}

pub fn random_quintic_pairs(
    count: usize,
) -> Vec<(QuinticTrinomialExtension, QuinticTrinomialExtension)> {
    let mut rng = XorShift64::new(VECTOR_SEED);
    (0..count).map(|_| (rng.next_quintic(), rng.next_quintic())).collect()
}

pub fn generate_ext5_arithmetic_vectors(count: usize) -> Ext5ArithmeticVectorFile {
    let mut rng = XorShift64::new(VECTOR_SEED);
    let mut vectors = Vec::with_capacity(count);
    for _ in 0..count {
        let a = rng.next_quintic();
        let b = rng.next_quintic();
        let scalar = rng.next_base_field();
        vectors.push(Ext5ArithmeticVector {
            packed_a: encode_hex_word(&encode_packed_ext5_word(&a)),
            packed_b: encode_hex_word(&encode_packed_ext5_word(&b)),
            scalar: format!("0x{:064x}", scalar.as_canonical_u32()),
            packed_add: encode_hex_word(&encode_packed_ext5_word(&(a + b))),
            packed_sub: encode_hex_word(&encode_packed_ext5_word(&(a - b))),
            packed_mul: encode_hex_word(&encode_packed_ext5_word(&(a * b))),
            packed_square_a: encode_hex_word(&encode_packed_ext5_word(&(a * a))),
            packed_mul_base: encode_hex_word(&encode_packed_ext5_word(&(a * scalar))),
        });
    }
    Ext5ArithmeticVectorFile { seed: VECTOR_SEED, count, vectors }
}

pub fn write_vector_outputs(out_dir: &Path, count: usize) -> anyhow::Result<()> {
    let file = generate_ext5_arithmetic_vectors(count);
    let json_path = out_dir.join("ext5_precompile_vectors.json");
    let abi_path = out_dir.join("ext5_precompile_vectors.abi");

    let encoded = serde_json::to_vec_pretty(&file)?;
    fs::write(&json_path, encoded)
        .with_context(|| format!("failed to write {}", json_path.display()))?;

    let abi = Ext5ArithmeticVectorsAbi {
        packedA: file
            .vectors
            .iter()
            .map(|vector| parse_hex_u256(&vector.packed_a))
            .collect::<anyhow::Result<Vec<_>>>()?,
        packedB: file
            .vectors
            .iter()
            .map(|vector| parse_hex_u256(&vector.packed_b))
            .collect::<anyhow::Result<Vec<_>>>()?,
        scalars: file
            .vectors
            .iter()
            .map(|vector| parse_hex_u256(&vector.scalar))
            .collect::<anyhow::Result<Vec<_>>>()?,
        packedAdd: file
            .vectors
            .iter()
            .map(|vector| parse_hex_u256(&vector.packed_add))
            .collect::<anyhow::Result<Vec<_>>>()?,
        packedSub: file
            .vectors
            .iter()
            .map(|vector| parse_hex_u256(&vector.packed_sub))
            .collect::<anyhow::Result<Vec<_>>>()?,
        packedMul: file
            .vectors
            .iter()
            .map(|vector| parse_hex_u256(&vector.packed_mul))
            .collect::<anyhow::Result<Vec<_>>>()?,
        packedSquareA: file
            .vectors
            .iter()
            .map(|vector| parse_hex_u256(&vector.packed_square_a))
            .collect::<anyhow::Result<Vec<_>>>()?,
        packedMulBase: file
            .vectors
            .iter()
            .map(|vector| parse_hex_u256(&vector.packed_mul_base))
            .collect::<anyhow::Result<Vec<_>>>()?,
    };
    fs::write(&abi_path, abi.abi_encode())
        .with_context(|| format!("failed to write {}", abi_path.display()))?;
    Ok(())
}

fn parse_hex_u256(word: &str) -> anyhow::Result<U256> {
    let bytes = hex::decode(word.strip_prefix("0x").unwrap_or(word))?;
    let len = bytes.len();
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("expected 32-byte word, got {} bytes", len))?;
    Ok(U256::from_be_bytes(array))
}
