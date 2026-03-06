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
        let script_path = Self::resolve_script_path()?;
        
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
        let mut path = std::env::current_exe()
            .context("Failed to get current executable path")?;
        
        // Walk up from target/release/comes to root
        // comes -> release -> target -> root
        for _ in 0..3 {
            if let Some(parent) = path.parent() {
                path = parent.to_path_buf();
            } else {
                anyhow::bail!("Could not find root directory from executable path");
            }
        }
        
        let script_path = path.join("scripts").join("audio_analysis.py");
        if !script_path.exists() {
            // Fallback for development (target/debug/comes)
            // Sometimes current_exe might be in target/debug/deps/ during tests
            // but for now we follow the 3-level rule or just check if it exists
            anyhow::bail!("Script not found at: {:?}", script_path);
        }
        
        Ok(script_path)
    }
}
