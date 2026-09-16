//! Processor latency harness (no external bench framework).
//!
//! Drives the same [`TextIntelProcessor`] code the host invokes per chat
//! message and reports p50/p95/p99 per input class. Run with:
//!
//! ```sh
//! cargo bench --bench processor_latency
//! ```
//!
//! Use the numbers to justify the manifest `timeoutMs`, never as a quality
//! claim: TTS model preparation and other heavy work must not share this
//! plugin process.

use std::time::{Duration, Instant};

use serde_json::Value;
use tiktools_textintel_plugin::{EngineMode, TextIntelProcessor};

fn percentile(mut samples: Vec<Duration>, percentile: f64) -> Duration {
    samples.sort_unstable();
    let index = ((samples.len() as f64 - 1.0) * percentile).round() as usize;
    samples[index.min(samples.len().saturating_sub(1))]
}

fn bench_case(
    processor: &mut TextIntelProcessor,
    settings: &Value,
    name: &str,
    comment: &str,
    nickname: &str,
    iterations: usize,
    unique_inputs: bool,
) {
    // Fresh plugin caches per iteration so every sample measures analysis.
    // `unique_inputs` additionally defeats the engine's internal caches with
    // a per-iteration suffix, measuring cold first-seen text.
    let mut logs = Vec::new();
    for _ in 0..10 {
        processor.clear_caches();
        let _ = processor.enrich_texts(settings, comment, nickname, "bench_user", &mut logs);
    }
    let mut samples = Vec::with_capacity(iterations);
    for iteration in 0..iterations {
        processor.clear_caches();
        let cold_comment;
        let cold_nickname;
        let (comment, nickname) = if unique_inputs {
            cold_comment = format!("{comment} #{iteration:06}");
            cold_nickname = format!("{nickname}{iteration:04}");
            (cold_comment.as_str(), cold_nickname.as_str())
        } else {
            (comment, nickname)
        };
        let started = Instant::now();
        let _ = processor.enrich_texts(settings, comment, nickname, "bench_user", &mut logs);
        samples.push(started.elapsed());
    }
    let mean_micros: f64 = samples
        .iter()
        .map(|sample| sample.as_micros() as f64)
        .sum::<f64>()
        / samples.len() as f64;
    println!(
        "{:<22} mean {:>8.0}µs  p50 {:>8}µs  p95 {:>8}µs  p99 {:>8}µs",
        name,
        mean_micros,
        percentile(samples.clone(), 0.50).as_micros(),
        percentile(samples.clone(), 0.95).as_micros(),
        percentile(samples, 0.99).as_micros(),
    );
}

fn main() {
    for mode in [EngineMode::Default, EngineMode::ProductionLocalLite] {
        // Settings travel as host JSON; defaults fill every unset field.
        let settings = serde_json::json!({"engineMode": mode.as_str()});
        let mut processor = TextIntelProcessor::new();
        let mut logs = Vec::new();
        if processor
            .enrich_texts(&settings, "warmup", "Viewer", "viewer", &mut logs)
            .is_err()
        {
            println!("mode {} unavailable, skipped", mode.as_str());
            continue;
        }
        println!("engine mode: {}", mode.as_str());
        let cases = [
            ("short-ascii", "hello".to_owned(), "Viewer".to_owned(), 200),
            ("emoji-only", "😂😂😂".to_owned(), "Viewer".to_owned(), 200),
            (
                "spanish",
                "hola a todos, ¿cómo están esta noche?".to_owned(),
                "J0sé".to_owned(),
                200,
            ),
            (
                "mixed-unicode",
                "Fra🏠do salU2 日本語 مرحبا".to_owned(),
                "J0sé_92".to_owned(),
                200,
            ),
            (
                "obfuscated",
                "c0mpr4 ah0r4!!! visit https://example.com".to_owned(),
                "xX_Viewer_Xx".to_owned(),
                200,
            ),
            (
                "comment+nickname",
                "HOOOLAAA 😂😂".to_owned(),
                "María José García Fernández".to_owned(),
                200,
            ),
            (
                "max-input",
                "hola mundo 😂 ".repeat(40),
                "n".repeat(120),
                25,
            ),
        ];
        for (name, comment, nickname, iterations) in &cases {
            bench_case(
                &mut processor,
                &settings,
                name,
                comment,
                nickname,
                *iterations,
                false,
            );
        }
        println!("-- cold (unique inputs, engine caches defeated) --");
        for (name, comment, nickname, iterations) in &cases {
            bench_case(
                &mut processor,
                &settings,
                &format!("{name}-cold"),
                comment,
                nickname,
                (*iterations).min(50),
                true,
            );
        }
        println!();
    }
}
