use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::path::PathBuf;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct AudioFeatures {
    pub duration_seconds: f64,
    pub wpm_estimate: u32,
    pub pause_count: u32,
    pub pause_ratio: f64,
    pub pitch_mean_hz: f64,
    pub pitch_std_hz: f64,
    pub pitch_variation: String,
}

#[allow(dead_code)]
pub struct AudioProcessor;

#[allow(dead_code)]
impl AudioProcessor {
    // Run ffmpeg to convert OGG to WAV
    pub fn convert_to_wav(input_ogg: &str, output_wav: &str) -> Result<()> {
        let status = Command::new("ffmpeg")
            .arg("-i")
            .arg(input_ogg)
            .arg("-ar")
            .arg("16000")
            .arg("-ac")
            .arg("1")
            .arg(output_wav)
            .arg("-y")
            .arg("-loglevel")
            .arg("error")
            .status()
            .context("Failed to execute ffmpeg")?;

        if !status.success() {
            anyhow::bail!("ffmpeg failed with status: {}", status);
        }
        Ok(())
    }

    // Run scripts/audio_analysis.py via `uv run --script` subprocess
    // Returns parsed AudioFeatures or error
    pub fn analyse(wav_path: &str) -> Result<AudioFeatures> {
        let script_path = Self::resolve_script_path()
            .context("Could not locate audio_analysis.py. Set PHRON_SCRIPTS_DIR env var or run from repo root.")?;
        
        let output = Command::new("uv")
            .arg("run")
            .arg("--script")
            .arg(script_path)
            .arg(wav_path)
            .output()
            .context("Failed to execute uv run --script")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Audio analysis script failed: {}", stderr);
        }

        let features: AudioFeatures = serde_json::from_slice(&output.stdout)
            .context("Failed to parse audio analysis output")?;

        Ok(features)
    }

    // Clean up temp files
    pub fn cleanup(paths: &[&str]) {
        for path in paths {
            let _ = std::fs::remove_file(path);
        }
    }

    fn resolve_script_path() -> Result<PathBuf> {
        // 1. Explicit env var — set this in LaunchAgent or shell for installed binary
        if let Ok(dir) = std::env::var("PHRON_SCRIPTS_DIR") {
            let p = PathBuf::from(dir).join("audio_analysis.py");
            if p.exists() { return Ok(p); }
        }

        // 2. Walk up from exe — works for target/release/comes-bot (dev)
        if let Ok(exe) = std::env::current_exe() {
            for depth in 1..=4 {
                let mut candidate = exe.clone();
                for _ in 0..depth { candidate.pop(); }
                let p = candidate.join("scripts").join("audio_analysis.py");
                if p.exists() { return Ok(p); }
            }
        }

        // 3. Hardcoded repo path fallback
        let fallback = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("scripts")
            .join("audio_analysis.py");
        if fallback.exists() { return Ok(fallback); }

        anyhow::bail!(
            "audio_analysis.py not found. Set PHRON_SCRIPTS_DIR=~/code/phron/scripts in your environment."
        )
    }
}
