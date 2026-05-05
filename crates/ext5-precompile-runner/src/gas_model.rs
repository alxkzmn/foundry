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
    addresses::EXTFIELD_MAC_FIELD_ID_KOALABEAR_EXT5,
    field_types::{QuinticTrinomialExtension, F},
    vectors::{random_quintic_pairs, VECTOR_SEED},
};

pub const EIP1108_GAS_PER_MICROSECOND: f64 = 25.86;
pub const SAFETY_MULTIPLIER: f64 = 8.0;
pub const MIN_REALISTIC_EFFECTIVE_GAS: u64 = 700;
pub const ROUND_TO_GAS: u64 = 50;
pub const BENCH_SAMPLES: usize = 101;
pub const OPS_PER_SAMPLE: usize = 1024;
pub const EXTFIELD_MAC_N_MAX: usize = 1024;
pub const EXTFIELD_MAC_CALIBRATION_NS: [usize; 5] = [0, 1, 16, 64, EXTFIELD_MAC_N_MAX];

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
    pub ext5_mul: OperationGasModel,
    pub ext5_square: OperationGasModel,
    pub ext5_add: OperationGasModel,
    pub ext5_sub: OperationGasModel,
    pub ext5_mul_base: OperationGasModel,
    pub ext5_mul_batch: OperationGasModel,
    pub ext5_square_batch: OperationGasModel,
    pub ext5_mul_base_batch: OperationGasModel,
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
pub struct ExtfieldMacGasSchedule {
    pub foundry_version: String,
    pub foundry_commit: String,
    pub eip1108_gas_per_microsecond: f64,
    pub safety_multiplier: f64,
    pub min_realistic_effective_gas: u64,
    pub samples: usize,
    pub ops_per_sample: usize,
    pub vector_seed: u64,
    pub field_id: u16,
    pub n_max: usize,
    pub extfield_mac: ExtfieldMacGasModel,
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
            ext5_mul: OperationGasModel {
                median_runtime_ns: mul_median,
                assigned_base_gas: assigned_base_gas_from_ns(mul_median),
            },
            ext5_square: OperationGasModel {
                median_runtime_ns: square_median,
                assigned_base_gas: assigned_base_gas_from_ns(square_median),
            },
            ext5_add: OperationGasModel {
                median_runtime_ns: add_median,
                assigned_base_gas: assigned_base_gas_from_ns(add_median),
            },
            ext5_sub: OperationGasModel {
                median_runtime_ns: sub_median,
                assigned_base_gas: assigned_base_gas_from_ns(sub_median),
            },
            ext5_mul_base: OperationGasModel {
                median_runtime_ns: mul_base_median,
                assigned_base_gas: assigned_base_gas_from_ns(mul_base_median),
            },
            ext5_mul_batch: OperationGasModel {
                median_runtime_ns: mul_batch_median,
                assigned_base_gas: assigned_base_gas_from_ns(mul_batch_median),
            },
            ext5_square_batch: OperationGasModel {
                median_runtime_ns: square_batch_median,
                assigned_base_gas: assigned_base_gas_from_ns(square_batch_median),
            },
            ext5_mul_base_batch: OperationGasModel {
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
    pub fn from_live_benchmark() -> Self {
        let samples = EXTFIELD_MAC_CALIBRATION_NS
            .iter()
            .map(|n| {
                let median_runtime_ns = benchmark_mac_ns(*n);
                MacSizeSample {
                    n: *n,
                    median_runtime_ns,
                    assigned_gas_at_n: assigned_base_gas_from_ns(median_runtime_ns),
                }
            })
            .collect::<Vec<_>>();
        let (assigned_base_gas, assigned_per_pair_gas) = fit_mac_gas(&samples);
        Self {
            foundry_version: "1.5.1-stable".to_string(),
            foundry_commit: "b0a9dd9ceda36f63e2326ce530c10e6916f4b8a2".to_string(),
            eip1108_gas_per_microsecond: EIP1108_GAS_PER_MICROSECOND,
            safety_multiplier: SAFETY_MULTIPLIER,
            min_realistic_effective_gas: MIN_REALISTIC_EFFECTIVE_GAS,
            samples: BENCH_SAMPLES,
            ops_per_sample: OPS_PER_SAMPLE,
            vector_seed: VECTOR_SEED,
            field_id: EXTFIELD_MAC_FIELD_ID_KOALABEAR_EXT5,
            n_max: EXTFIELD_MAC_N_MAX,
            extfield_mac: ExtfieldMacGasModel { assigned_base_gas, assigned_per_pair_gas, samples },
        }
    }

    pub fn write_json(&self, path: &Path) -> anyhow::Result<()> {
        let encoded = serde_json::to_vec_pretty(self)?;
        fs::write(path, encoded)
            .with_context(|| format!("failed to write MAC gas schedule {}", path.display()))
    }

    pub fn read_json(path: &Path) -> anyhow::Result<Self> {
        let raw = fs::read(path)
            .with_context(|| format!("failed to read MAC gas schedule {}", path.display()))?;
        Ok(serde_json::from_slice(&raw)?)
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
    mut op: impl FnMut(
        &QuinticTrinomialExtension,
        &QuinticTrinomialExtension,
    ) -> QuinticTrinomialExtension,
) -> u64 {
    let inputs = random_quintic_pairs(BENCH_SAMPLES * OPS_PER_SAMPLE + 1);
    let mut samples = Vec::with_capacity(BENCH_SAMPLES);
    let mut cursor = 0usize;

    for _ in 0..BENCH_SAMPLES {
        let start = Instant::now();
        let mut acc = QuinticTrinomialExtension::ZERO;
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

fn benchmark_square_ns(
    mut op: impl FnMut(&QuinticTrinomialExtension) -> QuinticTrinomialExtension,
) -> u64 {
    let inputs = random_quintic_pairs(BENCH_SAMPLES * OPS_PER_SAMPLE + 1);
    let mut samples = Vec::with_capacity(BENCH_SAMPLES);
    let mut cursor = 0usize;

    for _ in 0..BENCH_SAMPLES {
        let start = Instant::now();
        let mut acc = QuinticTrinomialExtension::ZERO;
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

fn benchmark_scalar_op_ns(
    mut op: impl FnMut(&QuinticTrinomialExtension, &F) -> QuinticTrinomialExtension,
) -> u64 {
    let inputs = random_quintic_pairs(BENCH_SAMPLES * OPS_PER_SAMPLE + 1);
    let mut samples = Vec::with_capacity(BENCH_SAMPLES);
    let mut cursor = 0usize;

    for _ in 0..BENCH_SAMPLES {
        let start = Instant::now();
        let mut acc = QuinticTrinomialExtension::ZERO;
        for idx in 0..OPS_PER_SAMPLE {
            let (a, b) = &inputs[cursor + idx];
            let scalar =
                <QuinticTrinomialExtension as BasedVectorSpace<F>>::as_basis_coefficients_slice(b)
                    [0];
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
    mut op: impl FnMut(
        &QuinticTrinomialExtension,
        &QuinticTrinomialExtension,
    ) -> QuinticTrinomialExtension,
) -> u64 {
    benchmark_op_ns(|a, b| op(a, b))
}

fn benchmark_batch_square_ns(
    mut op: impl FnMut(&QuinticTrinomialExtension) -> QuinticTrinomialExtension,
) -> u64 {
    benchmark_square_ns(|a| op(a))
}

fn benchmark_batch_scalar_op_ns(
    mut op: impl FnMut(&QuinticTrinomialExtension, &F) -> QuinticTrinomialExtension,
) -> u64 {
    benchmark_scalar_op_ns(|a, scalar| op(a, scalar))
}

fn benchmark_mac_ns(n: usize) -> u64 {
    let inputs = random_quintic_pairs(n.max(1));
    let accumulator = inputs[0].0;
    let mut samples = Vec::with_capacity(BENCH_SAMPLES);

    for _ in 0..BENCH_SAMPLES {
        let start = Instant::now();
        let mut outer_acc = QuinticTrinomialExtension::ZERO;
        for _ in 0..OPS_PER_SAMPLE {
            let mut acc = black_box(accumulator);
            for (a, b) in inputs.iter().take(n) {
                acc += black_box(*a) * black_box(*b);
            }
            outer_acc += black_box(acc);
        }
        let _ = black_box(outer_acc);
        let elapsed = start.elapsed();
        samples.push(ns_per_op(elapsed));
    }

    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn fit_mac_gas(samples: &[MacSizeSample]) -> (u64, u64) {
    let count = samples.len() as f64;
    let sum_x = samples.iter().map(|sample| sample.n as f64).sum::<f64>();
    let sum_y = samples.iter().map(|sample| sample.median_runtime_ns as f64).sum::<f64>();
    let sum_xx = samples
        .iter()
        .map(|sample| {
            let n = sample.n as f64;
            n * n
        })
        .sum::<f64>();
    let sum_xy =
        samples.iter().map(|sample| sample.n as f64 * sample.median_runtime_ns as f64).sum::<f64>();
    let denom = count * sum_xx - sum_x * sum_x;
    let slope = if denom == 0.0 { 0.0 } else { (count * sum_xy - sum_x * sum_y) / denom };
    let intercept = (sum_y - slope * sum_x) / count;
    let gas_per_ns = SAFETY_MULTIPLIER * EIP1108_GAS_PER_MICROSECOND / 1_000.0;
    let base = ceil_to(intercept.max(0.0) * gas_per_ns, ROUND_TO_GAS).max(ROUND_TO_GAS);
    let per_pair = (slope.max(0.0) * gas_per_ns).ceil().max(1.0) as u64;
    (base, per_pair)
}

fn ns_per_op(duration: Duration) -> u64 {
    let nanos = duration.as_nanos() as f64 / OPS_PER_SAMPLE as f64;
    nanos.round() as u64
}
