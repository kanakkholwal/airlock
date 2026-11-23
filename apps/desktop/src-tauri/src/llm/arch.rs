use super::engine::Tuning;
use crate::managers::gguf_meta::{self, GgufMetadata, GgufValue};
use std::path::Path;

/// What decides a model's context and chat-cache cost, from its catalog entry or GGUF header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelArch {
    pub max_context: u32,
    /// Chat cache per token at 16-bit, full-attention layers only.
    pub kv_kib_per_token: u32,
    /// Fixed cache for sliding-window layers.
    pub kv_fixed_mib: u32,
}

/// Smallest chat length worth loading a model for.
pub const MIN_CONTEXT: u32 = 4096;
/// Largest chat length chosen automatically; longer stays a manual choice.
const AUTO_CONTEXT_CAP: u32 = 262_144;
/// Share of usable memory that weights plus cache may take, leaving room for the system.
const AUTO_MEMORY_SHARE: f64 = 0.75;
/// 8-bit cache size relative to 16-bit (q8_0 is 8.5 bits per value).
const KV_Q8_RATIO: f64 = 8.5 / 16.0;
pub const WORK_RESERVE: u64 = 256 * 1024 * 1024;

fn uint(v: &GgufValue) -> Option<u64> {
    match v {
        GgufValue::U8(n) => Some(*n as u64),
        GgufValue::U16(n) => Some(*n as u64),
        GgufValue::U32(n) => Some(*n as u64),
        GgufValue::U64(n) => Some(*n),
        GgufValue::I32(n) => u64::try_from(*n).ok(),
        GgufValue::I64(n) => u64::try_from(*n).ok(),
        _ => None,
    }
}

/// Reads the architecture facts from a GGUF header on disk; `None` when it is unreadable or
/// lacks them, in which case callers fall back to a flat guess.
pub fn from_gguf(path: &Path) -> Option<ModelArch> {
    let head = gguf_meta::read_header(path, &["general.architecture"]).ok()?;
    let arch = head.get_str("general.architecture")?.to_string();
    let keys: Vec<String> = [
        "context_length",
        "block_count",
        "embedding_length",
        "attention.head_count",
        "attention.head_count_kv",
        "attention.key_length",
        "attention.value_length",
        "full_attention_interval",
        "attention.sliding_window",
        "attention.sliding_window_pattern",
        "attention.key_length_swa",
        "attention.value_length_swa",
    ]
    .iter()
    .map(|k| format!("{arch}.{k}"))
    .collect();
    let wanted: Vec<&str> = keys.iter().map(String::as_str).collect();
    from_metadata(&arch, &gguf_meta::read_header(path, &wanted).ok()?)
}

/// Decision (architecture): layers with a cache are the non-zero entries of a per-layer
/// head-count array, or every Nth layer when `full_attention_interval` is set. Layers the
/// `sliding_window_pattern` marks (1) keep only one window, so they cost a fixed amount, not
/// a per-token one. Shared-KV layers are not subtracted, so the estimate errs high.
pub fn from_metadata(arch: &str, meta: &GgufMetadata) -> Option<ModelArch> {
    let get = |k: &str| meta.kv.get(&format!("{arch}.{k}"));
    let num = |k: &str| get(k).and_then(uint);
    let max_context = u32::try_from(num("context_length")?).ok()?;
    let layers = num("block_count")? as usize;
    let heads = num("attention.head_count").or_else(|| match get("attention.head_count")? {
        GgufValue::Array(a) => a.iter().filter_map(uint).max(),
        _ => None,
    });
    // Per layer: KV head count, 0 when the layer has no cache.
    let kv_heads: Vec<u64> = match get("attention.head_count_kv")? {
        GgufValue::Array(a) => a.iter().map(|v| uint(v).unwrap_or(0)).collect(),
        v => {
            let h = uint(v)?;
            let every = num("full_attention_interval")
                .filter(|i| *i > 0)
                .unwrap_or(1) as usize;
            (0..layers)
                .map(|l| if (l + 1) % every == 0 { h } else { 0 })
                .collect()
        }
    };
    let sliding: Vec<bool> = match get("attention.sliding_window_pattern") {
        Some(GgufValue::Array(a)) => a
            .iter()
            .map(|v| uint(v).unwrap_or(0) == 1 || matches!(v, GgufValue::Bool(true)))
            .collect(),
        _ => vec![],
    };
    let head_dim = num("attention.key_length").or_else(|| {
        let h = heads.filter(|h| *h > 0)?;
        Some(num("embedding_length")? / h)
    })?;
    let value_dim = num("attention.value_length").unwrap_or(head_dim);
    let head_dim_swa = num("attention.key_length_swa").unwrap_or(head_dim);
    let value_dim_swa = num("attention.value_length_swa").unwrap_or(value_dim);
    let window = num("attention.sliding_window").unwrap_or(0);

    let (mut per_token, mut fixed) = (0u64, 0u64);
    for (l, h) in kv_heads.iter().enumerate().filter(|(_, h)| **h > 0) {
        if sliding.get(l).copied().unwrap_or(false) && window > 0 {
            fixed += h * (head_dim_swa + value_dim_swa) * 2 * window;
        } else {
            per_token += h * (head_dim + value_dim) * 2;
        }
    }
    Some(ModelArch {
        max_context,
        kv_kib_per_token: per_token.div_ceil(1024) as u32,
        kv_fixed_mib: fixed.div_ceil(1 << 20) as u32,
    })
}

impl ModelArch {
    pub fn cache_bytes(&self, context: u32, tuning: &Tuning) -> u64 {
        let raw = context as u64 * self.kv_kib_per_token as u64 * 1024
            + self.kv_fixed_mib as u64 * 1024 * 1024;
        if tuning.flash_attn && tuning.kv_q8 {
            (raw as f64 * KV_Q8_RATIO) as u64
        } else {
            raw
        }
    }

    /// Weights, cache at `context`, and a fixed work reserve.
    pub fn memory_bytes(&self, weights: u64, context: u32, tuning: &Tuning) -> u64 {
        weights + self.cache_bytes(context, tuning) + WORK_RESERVE
    }

    /// Decision (product): chat length is automatic, like a hosted assistant: the model's full
    /// window when memory allows, otherwise the longest that fits. A user override replaces it.
    pub fn auto_context(&self, weights: u64, usable_bytes: u64, tuning: &Tuning) -> u32 {
        let budget = (usable_bytes as f64 * AUTO_MEMORY_SHARE) as u64;
        let top = self.max_context.min(AUTO_CONTEXT_CAP);
        let mut ctx = top;
        while ctx > MIN_CONTEXT && self.memory_bytes(weights, ctx, tuning) > budget {
            ctx = (ctx / 2).max(MIN_CONTEXT);
        }
        ctx.min(top).max(MIN_CONTEXT.min(top))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn meta(pairs: Vec<(&str, GgufValue)>) -> GgufMetadata {
        GgufMetadata {
            kv: pairs
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect::<HashMap<_, _>>(),
        }
    }

    #[test]
    fn hybrid_model_counts_only_cache_layers() {
        // Qwen3.5 4B: 32 layers, one in four has a cache, 4 KV heads of 256.
        let m = meta(vec![
            ("qwen35.context_length", GgufValue::U32(262144)),
            ("qwen35.block_count", GgufValue::U32(32)),
            ("qwen35.attention.head_count", GgufValue::U32(16)),
            ("qwen35.attention.head_count_kv", GgufValue::U32(4)),
            ("qwen35.attention.key_length", GgufValue::U32(256)),
            ("qwen35.full_attention_interval", GgufValue::U32(4)),
        ]);
        let a = from_metadata("qwen35", &m).unwrap();
        assert_eq!((a.max_context, a.kv_kib_per_token), (262144, 32));
    }

    #[test]
    fn per_layer_array_skips_layers_without_attention() {
        let m = meta(vec![
            ("lfm3.context_length", GgufValue::U32(262144)),
            ("lfm3.block_count", GgufValue::U32(24)),
            ("lfm3.attention.head_count", GgufValue::U32(16)),
            (
                "lfm3.attention.head_count_kv",
                GgufValue::Array(
                    (0..24)
                        .map(|i| GgufValue::U32(if i % 4 == 0 { 4 } else { 0 }))
                        .collect(),
                ),
            ),
            ("lfm3.attention.key_length", GgufValue::U32(128)),
            ("lfm3.attention.sliding_window", GgufValue::U32(256)),
            (
                "lfm3.attention.sliding_window_pattern",
                GgufValue::Array(
                    (0..24)
                        .map(|i| GgufValue::U32(u32::from(i % 8 == 4)))
                        .collect(),
                ),
            ),
        ]);
        // Layers 0, 4, ..., 20 have a cache; 4, 12 and 20 slide. 3 full x 4 heads x 256 x 2 bytes = 6 KiB.
        let a = from_metadata("lfm3", &m).unwrap();
        assert_eq!((a.kv_kib_per_token, a.kv_fixed_mib), (6, 2));
    }

    #[test]
    fn auto_context_shrinks_to_memory_and_never_below_the_floor() {
        let a = ModelArch {
            max_context: 262144,
            kv_kib_per_token: 32,
            kv_fixed_mib: 0,
        };
        let t = Tuning::default();
        let gib = 1u64 << 30;
        assert_eq!(a.auto_context(3 * gib, 64 * gib, &t), 262144);
        let tight = a.auto_context(3 * gib, 6 * gib, &t);
        assert!(tight >= MIN_CONTEXT && tight < 262144);
        assert_eq!(a.auto_context(10 * gib, 4 * gib, &t), MIN_CONTEXT);
        let small = ModelArch {
            max_context: 2048,
            ..a
        };
        assert_eq!(small.auto_context(gib, 64 * gib, &t), 2048);
    }

    /// Run by hand against a real file: `AIRLOCK_TEST_GGUF=path cargo test --lib real_gguf -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_gguf() {
        let path = std::env::var("AIRLOCK_TEST_GGUF").expect("set AIRLOCK_TEST_GGUF");
        println!("{:?}", from_gguf(Path::new(&path)));
    }
}
