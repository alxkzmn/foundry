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
pub const DEFAULT_MAC_VECTOR_REPEATS: usize = 16;
pub const MAC_VECTOR_LENGTHS: [usize; 4] = [0, 1, 16, 64];

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtfieldMacVector {
    pub n: usize,
    pub include_accumulator: bool,
    pub packed_accumulator: String,
    pub packed_a: Vec<String>,
    pub packed_b: Vec<String>,
    pub packed_output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtfieldMacVectorFile {
    pub seed: u64,
    pub repeats: usize,
    pub lengths: Vec<usize>,
    pub vectors: Vec<ExtfieldMacVector>,
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

pub fn generate_extfield_mac_vectors(repeats: usize) -> ExtfieldMacVectorFile {
    let mut rng = XorShift64::new(VECTOR_SEED ^ 0xA7AC_D017_5EED_0005);
    let mut vectors = Vec::with_capacity(MAC_VECTOR_LENGTHS.len() * 2 * repeats);
    for n in MAC_VECTOR_LENGTHS {
        for include_accumulator in [false, true] {
            for _ in 0..repeats {
                let accumulator = if include_accumulator {
                    rng.next_quintic()
                } else {
                    QuinticTrinomialExtension::ZERO
                };
                let mut acc = accumulator;
                let mut packed_a = Vec::with_capacity(n);
                let mut packed_b = Vec::with_capacity(n);
                for _ in 0..n {
                    let a = rng.next_quintic();
                    let b = rng.next_quintic();
                    acc += a * b;
                    packed_a.push(encode_hex_word(&encode_packed_ext5_word(&a)));
                    packed_b.push(encode_hex_word(&encode_packed_ext5_word(&b)));
                }
                vectors.push(ExtfieldMacVector {
                    n,
                    include_accumulator,
                    packed_accumulator: encode_hex_word(&encode_packed_ext5_word(&accumulator)),
                    packed_a,
                    packed_b,
                    packed_output: encode_hex_word(&encode_packed_ext5_word(&acc)),
                });
            }
        }
    }
    ExtfieldMacVectorFile {
        seed: VECTOR_SEED,
        repeats,
        lengths: MAC_VECTOR_LENGTHS.to_vec(),
        vectors,
    }
}

pub fn write_mac_vector_outputs(out_dir: &Path, repeats: usize) -> anyhow::Result<()> {
    let file = generate_extfield_mac_vectors(repeats);
    let json_path = out_dir.join("extfield_mac_vectors.json");
    let encoded = serde_json::to_vec_pretty(&file)?;
    fs::write(&json_path, encoded)
        .with_context(|| format!("failed to write {}", json_path.display()))?;
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
