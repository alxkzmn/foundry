use std::{
    fs,
    hint::black_box,
    path::Path,
    time::{Duration, Instant},
};

use anyhow::Context;
use p3_field::{BasedVectorSpace, PrimeCharacteristicRing};
use serde::{Deserialize, Serialize};

use crate::{
    field_types::{OcticBinExtension, F},
    vectors::{random_octic_pairs, VECTOR_SEED},
};

pub const EIP1108_GAS_PER_MICROSECOND: f64 = 25.86;
pub const SAFETY_MULTIPLIER: f64 = 8.0;
pub const MIN_REALISTIC_EFFECTIVE_GAS: u64 = 700;
pub const ROUND_TO_GAS: u64 = 50;
pub const BENCH_SAMPLES: usize = 101;
pub const OPS_PER_SAMPLE: usize = 1024;
pub const EXTFIELD_LIN_PROD_FLAG_EXPLICIT: u32 = 0;
pub const EXTFIELD_LIN_PROD_FLAG_ALPHA_ONE_EXT_BETA: u32 = 1;
pub const EXTFIELD_LIN_PROD_FLAG_ALPHA_ONE_BASE_BETA: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationGasModel {
    pub median_runtime_ns: u64,
    pub assigned_base_gas: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedGasSchedule {
    pub foundry_version: String,
    pub foundry_commit: String,
    pub eip1108_gas_per_microsecond: f64,
    pub safety_multiplier: f64,
    pub min_realistic_effective_gas: u64,
    pub samples: usize,
    pub ops_per_sample: usize,
    pub vector_seed: u64,
    pub ext8_mul: OperationGasModel,
    pub ext8_square: OperationGasModel,
    pub ext8_add: OperationGasModel,
    pub ext8_sub: OperationGasModel,
    pub ext8_mul_base: OperationGasModel,
    pub ext8_mul_batch: OperationGasModel,
    pub ext8_square_batch: OperationGasModel,
    pub ext8_mul_base_batch: OperationGasModel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacSizeSample {
    pub n: usize,
    pub median_runtime_ns: u64,
    pub assigned_gas_at_n: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtfieldMacGasModel {
    pub assigned_base_gas: u64,
    pub assigned_per_pair_gas: u64,
    pub samples: Vec<MacSizeSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtfieldMacFieldGasSchedule {
    pub field_id: u16,
    pub n_max: usize,
    pub extfield_mac: ExtfieldMacGasModel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtfieldMacGasSchedule {
    pub foundry_version: String,
    pub foundry_commit: String,
    pub eip1108_gas_per_microsecond: f64,
    pub safety_multiplier: f64,
    pub min_realistic_effective_gas: u64,
    pub samples: usize,
    pub ops_per_sample: usize,
    pub vector_seed: u64,
    pub fields: Vec<ExtfieldMacFieldGasSchedule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinProdSizeSample {
    pub n: usize,
    pub median_runtime_ns: u64,
    pub assigned_gas_at_n: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtfieldLinProdGasModel {
    pub flags: u32,
    pub assigned_base_gas: u64,
    pub assigned_per_term_gas: u64,
    pub samples: Vec<LinProdSizeSample>,
    pub measurement_samples: Vec<LinProdSizeSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtfieldLinProdFieldGasSchedule {
    pub field_id: u16,
    pub n_max: usize,
    pub modes: Vec<ExtfieldLinProdGasModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtfieldLinProdGasSchedule {
    pub foundry_version: String,
    pub foundry_commit: String,
    pub eip1108_gas_per_microsecond: f64,
    pub safety_multiplier: f64,
    pub min_realistic_effective_gas: u64,
    pub samples: usize,
    pub ops_per_sample: usize,
    pub vector_seed: u64,
    pub fields: Vec<ExtfieldLinProdFieldGasSchedule>,
}

impl LockedGasSchedule {
    pub fn from_live_benchmark() -> Self {
        let mul_median = benchmark_op_ns(|a, b| *a * *b);
        let square_median = benchmark_square_ns(|a| *a * *a);
        let add_median = benchmark_op_ns(|a, b| *a + *b);
        let sub_median = benchmark_op_ns(|a, b| *a - *b);
        let mul_base_median = benchmark_scalar_op_ns(|a, scalar| *a * *scalar);
        let mul_batch_median = benchmark_batch_op_ns(|a, b| *a * *b);
        let square_batch_median = benchmark_batch_square_ns(|a| *a * *a);
        let mul_base_batch_median = benchmark_batch_scalar_op_ns(|a, scalar| *a * *scalar);
        Self {
            foundry_version: "1.5.1-stable".to_string(),
            foundry_commit: "b0a9dd9ceda36f63e2326ce530c10e6916f4b8a2".to_string(),
            eip1108_gas_per_microsecond: EIP1108_GAS_PER_MICROSECOND,
            safety_multiplier: SAFETY_MULTIPLIER,
            min_realistic_effective_gas: MIN_REALISTIC_EFFECTIVE_GAS,
            samples: BENCH_SAMPLES,
            ops_per_sample: OPS_PER_SAMPLE,
            vector_seed: VECTOR_SEED,
            ext8_mul: OperationGasModel {
                median_runtime_ns: mul_median,
                assigned_base_gas: assigned_base_gas_from_ns(mul_median),
            },
            ext8_square: OperationGasModel {
                median_runtime_ns: square_median,
                assigned_base_gas: assigned_base_gas_from_ns(square_median),
            },
            ext8_add: OperationGasModel {
                median_runtime_ns: add_median,
                assigned_base_gas: assigned_base_gas_from_ns(add_median),
            },
            ext8_sub: OperationGasModel {
                median_runtime_ns: sub_median,
                assigned_base_gas: assigned_base_gas_from_ns(sub_median),
            },
            ext8_mul_base: OperationGasModel {
                median_runtime_ns: mul_base_median,
                assigned_base_gas: assigned_base_gas_from_ns(mul_base_median),
            },
            ext8_mul_batch: OperationGasModel {
                median_runtime_ns: mul_batch_median,
                assigned_base_gas: assigned_base_gas_from_ns(mul_batch_median),
            },
            ext8_square_batch: OperationGasModel {
                median_runtime_ns: square_batch_median,
                assigned_base_gas: assigned_base_gas_from_ns(square_batch_median),
            },
            ext8_mul_base_batch: OperationGasModel {
                median_runtime_ns: mul_base_batch_median,
                assigned_base_gas: assigned_base_gas_from_ns(mul_base_batch_median),
            },
        }
    }

    pub fn write_json(&self, path: &Path) -> anyhow::Result<()> {
        let encoded = serde_json::to_vec_pretty(self)?;
        fs::write(path, encoded)
            .with_context(|| format!("failed to write gas schedule {}", path.display()))
    }

    pub fn read_json(path: &Path) -> anyhow::Result<Self> {
        let raw = fs::read(path)
            .with_context(|| format!("failed to read gas schedule {}", path.display()))?;
        Ok(serde_json::from_slice(&raw)?)
    }
}

impl ExtfieldMacGasSchedule {
    pub fn read_json(path: &Path) -> anyhow::Result<Self> {
        let raw = fs::read(path)
            .with_context(|| format!("failed to read MAC gas schedule {}", path.display()))?;
        Ok(serde_json::from_slice(&raw)?)
    }

    pub fn field(&self, field_id: u16) -> Option<&ExtfieldMacFieldGasSchedule> {
        self.fields.iter().find(|field| field.field_id == field_id)
    }
}

impl ExtfieldLinProdGasSchedule {
    pub fn read_json(path: &Path) -> anyhow::Result<Self> {
        let raw = fs::read(path)
            .with_context(|| format!("failed to read LIN_PROD gas schedule {}", path.display()))?;
        Ok(serde_json::from_slice(&raw)?)
    }

    pub fn field(&self, field_id: u16) -> Option<&ExtfieldLinProdFieldGasSchedule> {
        self.fields.iter().find(|field| field.field_id == field_id)
    }

    pub fn mode(
        &self,
        field_id: u16,
        flags: u32,
    ) -> Option<(&ExtfieldLinProdFieldGasSchedule, &ExtfieldLinProdGasModel)> {
        let field = self.field(field_id)?;
        let mode = field.modes.iter().find(|mode| mode.flags == flags)?;
        Some((field, mode))
    }
}

pub fn assigned_base_gas_from_ns(median_runtime_ns: u64) -> u64 {
    let runtime_us = median_runtime_ns as f64 / 1_000.0;
    let raw = SAFETY_MULTIPLIER * EIP1108_GAS_PER_MICROSECOND * runtime_us;
    ceil_to(raw, ROUND_TO_GAS)
}

fn ceil_to(value: f64, step: u64) -> u64 {
    let step_f = step as f64;
    ((value / step_f).ceil() * step_f) as u64
}

fn benchmark_op_ns(
    mut op: impl FnMut(&OcticBinExtension, &OcticBinExtension) -> OcticBinExtension,
) -> u64 {
    let inputs = random_octic_pairs(BENCH_SAMPLES * OPS_PER_SAMPLE + 1);
    let mut samples = Vec::with_capacity(BENCH_SAMPLES);
    let mut cursor = 0usize;

    for _ in 0..BENCH_SAMPLES {
        let start = Instant::now();
        let mut acc = OcticBinExtension::ZERO;
        for idx in 0..OPS_PER_SAMPLE {
            let (a, b) = &inputs[cursor + idx];
            acc += black_box(op(black_box(a), black_box(b)));
        }
        let _ = black_box(acc);
        let elapsed = start.elapsed();
        samples.push(ns_per_op(elapsed));
        cursor += OPS_PER_SAMPLE;
    }

    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn benchmark_square_ns(mut op: impl FnMut(&OcticBinExtension) -> OcticBinExtension) -> u64 {
    let inputs = random_octic_pairs(BENCH_SAMPLES * OPS_PER_SAMPLE + 1);
    let mut samples = Vec::with_capacity(BENCH_SAMPLES);
    let mut cursor = 0usize;

    for _ in 0..BENCH_SAMPLES {
        let start = Instant::now();
        let mut acc = OcticBinExtension::ZERO;
        for idx in 0..OPS_PER_SAMPLE {
            let (a, _) = &inputs[cursor + idx];
            acc += black_box(op(black_box(a)));
        }
        let _ = black_box(acc);
        let elapsed = start.elapsed();
        samples.push(ns_per_op(elapsed));
        cursor += OPS_PER_SAMPLE;
    }

    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn benchmark_scalar_op_ns(mut op: impl FnMut(&OcticBinExtension, &F) -> OcticBinExtension) -> u64 {
    let inputs = random_octic_pairs(BENCH_SAMPLES * OPS_PER_SAMPLE + 1);
    let mut samples = Vec::with_capacity(BENCH_SAMPLES);
    let mut cursor = 0usize;

    for _ in 0..BENCH_SAMPLES {
        let start = Instant::now();
        let mut acc = OcticBinExtension::ZERO;
        for idx in 0..OPS_PER_SAMPLE {
            let (a, b) = &inputs[cursor + idx];
            let scalar =
                <OcticBinExtension as BasedVectorSpace<F>>::as_basis_coefficients_slice(b)[0];
            acc += black_box(op(black_box(a), black_box(&scalar)));
        }
        let _ = black_box(acc);
        let elapsed = start.elapsed();
        samples.push(ns_per_op(elapsed));
        cursor += OPS_PER_SAMPLE;
    }

    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn benchmark_batch_op_ns(
    mut op: impl FnMut(&OcticBinExtension, &OcticBinExtension) -> OcticBinExtension,
) -> u64 {
    benchmark_op_ns(|a, b| op(a, b))
}

fn benchmark_batch_square_ns(mut op: impl FnMut(&OcticBinExtension) -> OcticBinExtension) -> u64 {
    benchmark_square_ns(|a| op(a))
}

fn benchmark_batch_scalar_op_ns(
    mut op: impl FnMut(&OcticBinExtension, &F) -> OcticBinExtension,
) -> u64 {
    benchmark_scalar_op_ns(|a, scalar| op(a, scalar))
}

fn ns_per_op(duration: Duration) -> u64 {
    let nanos = duration.as_nanos() as f64 / OPS_PER_SAMPLE as f64;
    nanos.round() as u64
}
