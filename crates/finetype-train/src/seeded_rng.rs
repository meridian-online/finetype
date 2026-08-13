//! A seeded generator for the two draws candle makes on the training thread.
//!
//! candle 0.8's CPU backend takes every `Tensor::rand`/`Tensor::randn` from
//! `rand::rng()` — a per-thread generator seeded from the OS — and its
//! `set_seed` returns an error for `Device::Cpu`
//! (`candle-core/src/cpu_backend/mod.rs`: "cannot seed the CPU rng with
//! set_seed"). Parameter initialisation (`candle_nn::Init`) and dropout masks
//! (`candle_nn::ops::dropout`) are both such draws, so on CPU neither can be
//! reached by a seed the caller supplies.
//!
//! [`seed_thread`] installs a `StdRng` on the calling thread and the two entry
//! points below — [`dropout`] and [`seeded_var_builder`] — take their values
//! from it. Both fall back to candle's own behaviour when no generator is
//! installed, so a thread that never calls [`seed_thread`] is unchanged.
//!
//! The draws reproduce `candle_nn::Init::var`'s distributions, not its bytes:
//! Kaiming normal stays Kaiming normal with the same standard deviation. What
//! changes is that the sequence is a function of the seed.

use candle_core::{DType, Device, Result as CandleResult, Shape, Tensor, Var};
use candle_nn::init::{Init, NormalOrUniform};
use candle_nn::var_builder::SimpleBackend;
use candle_nn::{VarBuilder, VarMap};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::cell::RefCell;

thread_local! {
    static TRAINING_RNG: RefCell<Option<StdRng>> = const { RefCell::new(None) };
}

/// Restores the thread's previous generator when dropped.
pub struct SeedGuard {
    previous: Option<StdRng>,
}

impl Drop for SeedGuard {
    fn drop(&mut self) {
        TRAINING_RNG.with(|cell| *cell.borrow_mut() = self.previous.take());
    }
}

/// Draw this thread's parameter initialisation and dropout masks from a
/// generator seeded with `seed`, until the returned guard is dropped.
#[must_use = "dropping the guard immediately restores the previous generator"]
pub fn seed_thread(seed: u64) -> SeedGuard {
    let previous = TRAINING_RNG.with(|cell| cell.borrow_mut().replace(StdRng::seed_from_u64(seed)));
    SeedGuard { previous }
}

/// Whether a seeded generator is installed on this thread.
pub fn is_seeded() -> bool {
    TRAINING_RNG.with(|cell| cell.borrow().is_some())
}

fn with_rng<T>(f: impl FnOnce(&mut StdRng) -> T) -> Option<T> {
    TRAINING_RNG.with(|cell| cell.borrow_mut().as_mut().map(f))
}

/// Box–Muller, so the normal draws need no dependency outside `rand` 0.8.
fn standard_normal(rng: &mut StdRng) -> f64 {
    loop {
        let u1: f64 = rng.gen();
        if u1 > 0.0 {
            let u2: f64 = rng.gen();
            return (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
        }
    }
}

/// Dropout whose mask comes from this thread's seeded generator.
///
/// Falls back to `candle_nn::ops::dropout` when no generator is installed. The
/// mask matches candle's: elements whose draw is below `drop_p` are zeroed and
/// the rest are scaled by `1 / (1 - drop_p)`.
pub fn dropout(xs: &Tensor, drop_p: f32) -> CandleResult<Tensor> {
    if !(0. ..1.).contains(&drop_p) {
        candle_core::bail!("dropout probability has to be in [0, 1), got {drop_p}")
    }
    let n = xs.elem_count();
    let scale = 1.0 / (1.0 - drop_p);
    let mask = with_rng(|rng| {
        (0..n)
            .map(|_| {
                if rng.gen::<f32>() >= drop_p {
                    scale
                } else {
                    0.0
                }
            })
            .collect::<Vec<f32>>()
    });
    match mask {
        None => candle_nn::ops::dropout(xs, drop_p),
        Some(mask) => {
            let mask = Tensor::from_vec(mask, xs.shape(), xs.device())?.to_dtype(xs.dtype())?;
            xs * mask
        }
    }
}

/// The distribution `Init` draws from, with Kaiming already resolved to its
/// bounds or standard deviation.
enum Draw {
    Uniform { lo: f64, up: f64 },
    Normal { mean: f64, stdev: f64 },
}

/// Mirrors `candle_nn::Init::var`. `None` for `Init::Const`, which draws nothing.
fn draw_for(init: Init, shape: &Shape) -> Option<Draw> {
    match init {
        Init::Const(_) => None,
        Init::Uniform { lo, up } => Some(Draw::Uniform { lo, up }),
        Init::Randn { mean, stdev } => Some(Draw::Normal { mean, stdev }),
        Init::Kaiming {
            dist,
            fan,
            non_linearity,
        } => {
            let fan = fan.for_shape(shape);
            let stdev = non_linearity.gain() / (fan as f64).sqrt();
            Some(match dist {
                NormalOrUniform::Uniform => {
                    let bound = 3f64.sqrt() * stdev;
                    Draw::Uniform {
                        lo: -bound,
                        up: bound,
                    }
                }
                NormalOrUniform::Normal => Draw::Normal { mean: 0.0, stdev },
            })
        }
    }
}

/// The tensor `init` would produce, drawn from this thread's seeded generator.
/// `None` when no generator is installed or when `init` draws nothing.
fn seeded_init(
    shape: &Shape,
    init: Init,
    dtype: DType,
    device: &Device,
) -> CandleResult<Option<Tensor>> {
    let Some(draw) = draw_for(init, shape) else {
        return Ok(None);
    };
    let n = shape.elem_count();
    let values = with_rng(|rng| match draw {
        Draw::Uniform { lo, up } => (0..n)
            .map(|_| if up > lo { rng.gen_range(lo..up) } else { lo })
            .collect::<Vec<f64>>(),
        Draw::Normal { mean, stdev } => (0..n)
            .map(|_| mean + stdev * standard_normal(rng))
            .collect::<Vec<f64>>(),
    });
    match values {
        None => Ok(None),
        Some(values) => Ok(Some(
            Tensor::from_vec(values, shape, device)?.to_dtype(dtype)?,
        )),
    }
}

/// A `VarBuilder` backend that stores its variables in a `VarMap` exactly as
/// `VarBuilder::from_varmap` does, and initialises the new ones from this
/// thread's seeded generator.
struct SeededVarMap {
    varmap: VarMap,
}

impl SimpleBackend for SeededVarMap {
    fn get(
        &self,
        s: Shape,
        name: &str,
        h: Init,
        dtype: DType,
        dev: &Device,
    ) -> CandleResult<Tensor> {
        let mut data = self.varmap.data().lock().unwrap();
        if let Some(existing) = data.get(name) {
            if existing.shape() != &s {
                candle_core::bail!("shape mismatch on {name}: {s:?} <> {:?}", existing.shape())
            }
            return Ok(existing.as_tensor().clone());
        }
        let var = match seeded_init(&s, h, dtype, dev)? {
            Some(tensor) => Var::from_tensor(&tensor)?,
            None => h.var(s, dtype, dev)?,
        };
        let tensor = var.as_tensor().clone();
        data.insert(name.to_string(), var);
        Ok(tensor)
    }

    fn contains_tensor(&self, name: &str) -> bool {
        self.varmap.data().lock().unwrap().contains_key(name)
    }
}

/// A `VarBuilder` over `varmap` whose fresh variables are seeded.
///
/// Drop-in for `VarBuilder::from_varmap`: the variables land in the same
/// `VarMap`, so `all_vars`, `save` and `load` behave identically.
pub fn seeded_var_builder(varmap: &VarMap, dtype: DType, device: &Device) -> VarBuilder<'static> {
    VarBuilder::from_backend(
        Box::new(SeededVarMap {
            varmap: varmap.clone(),
        }),
        dtype,
        device.clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two seeded threads draw the same dropout mask; an unseeded one does not
    /// reach this code at all.
    #[test]
    fn dropout_mask_is_a_function_of_the_seed() {
        let device = Device::Cpu;
        let xs = Tensor::ones((4, 64), DType::F32, &device).unwrap();

        let a = {
            let _g = seed_thread(7);
            dropout(&xs, 0.5).unwrap().flatten_all().unwrap()
        };
        let b = {
            let _g = seed_thread(7);
            dropout(&xs, 0.5).unwrap().flatten_all().unwrap()
        };
        let c = {
            let _g = seed_thread(8);
            dropout(&xs, 0.5).unwrap().flatten_all().unwrap()
        };

        let a: Vec<f32> = a.to_vec1().unwrap();
        let b: Vec<f32> = b.to_vec1().unwrap();
        let c: Vec<f32> = c.to_vec1().unwrap();
        assert_eq!(a, b, "same seed must give the same mask");
        assert_ne!(a, c, "a different seed must give a different mask");
        // Not a constant mask: some elements dropped, some kept and scaled.
        assert!(a.iter().any(|v| *v == 0.0), "no element was dropped");
        assert!(a.iter().any(|v| *v != 0.0), "every element was dropped");
    }

    #[test]
    fn dropout_without_a_seed_still_drops() {
        assert!(!is_seeded());
        let device = Device::Cpu;
        let xs = Tensor::ones((4, 64), DType::F32, &device).unwrap();
        let out: Vec<f32> = dropout(&xs, 0.5)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();
        assert!(out.iter().any(|v| *v == 0.0), "no element was dropped");
        assert!(out.iter().any(|v| *v != 0.0), "every element was dropped");
    }

    /// The guard restores what was there before, so a nested seed cannot leak
    /// into the enclosing run.
    #[test]
    fn the_guard_restores_the_previous_generator() {
        assert!(!is_seeded());
        {
            let _outer = seed_thread(1);
            assert!(is_seeded());
            {
                let _inner = seed_thread(2);
                assert!(is_seeded());
            }
            assert!(is_seeded());
        }
        assert!(!is_seeded());
    }

    /// Weights initialised under the same seed are identical, and under a
    /// different seed are not.
    #[test]
    fn parameter_init_is_a_function_of_the_seed() {
        let device = Device::Cpu;
        let weights = |seed: u64| -> Vec<f32> {
            let _g = seed_thread(seed);
            let varmap = VarMap::new();
            let vb = seeded_var_builder(&varmap, DType::F32, &device);
            let linear = candle_nn::linear(16, 8, vb.pp("l")).unwrap();
            linear
                .weight()
                .flatten_all()
                .unwrap()
                .to_vec1::<f32>()
                .unwrap()
        };
        let a = weights(42);
        let b = weights(42);
        let c = weights(43);
        assert_eq!(a, b, "same seed must give the same weights");
        assert_ne!(a, c, "a different seed must give different weights");
        assert!(
            a.iter().any(|v| *v != 0.0),
            "kaiming init produced an all-zero weight"
        );
    }

    /// The seeded backend registers its variables in the `VarMap` it was given,
    /// so the optimizer and the checkpoint writer see them.
    #[test]
    fn seeded_variables_land_in_the_varmap() {
        let device = Device::Cpu;
        let _g = seed_thread(3);
        let varmap = VarMap::new();
        let vb = seeded_var_builder(&varmap, DType::F32, &device);
        let _ = candle_nn::linear(16, 8, vb.pp("l")).unwrap();
        let names: Vec<String> = varmap.data().lock().unwrap().keys().cloned().collect();
        assert_eq!(
            names.len(),
            2,
            "expected a weight and a bias, got {names:?}"
        );
        assert!(names.iter().any(|n| n == "l.weight"), "{names:?}");
        assert!(names.iter().any(|n| n == "l.bias"), "{names:?}");
    }

    /// Kaiming-normal draws keep candle's standard deviation: gain / sqrt(fan_in),
    /// which is sqrt(2/fan_in) for ReLU.
    #[test]
    fn kaiming_normal_keeps_candles_standard_deviation() {
        let device = Device::Cpu;
        let _g = seed_thread(11);
        let varmap = VarMap::new();
        let vb = seeded_var_builder(&varmap, DType::F32, &device);
        let fan_in = 512usize;
        let linear = candle_nn::linear(fan_in, 256, vb.pp("l")).unwrap();
        let w: Vec<f32> = linear
            .weight()
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        let n = w.len() as f64;
        let mean = w.iter().map(|v| *v as f64).sum::<f64>() / n;
        let var = w.iter().map(|v| (*v as f64 - mean).powi(2)).sum::<f64>() / n;
        let expected = (2.0f64 / fan_in as f64).sqrt();
        let ratio = var.sqrt() / expected;
        assert!(
            (0.9..1.1).contains(&ratio),
            "kaiming-normal sd {} is not within 10% of {expected}",
            var.sqrt()
        );
    }
}
