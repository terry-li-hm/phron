use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

        Self::parse_audio_features(&output.stdout)
    }

    fn parse_audio_features(stdout: &[u8]) -> Result<AudioFeatures> {
        serde_json::from_slice(stdout).context("Failed to parse audio analysis output")
    }

    // Clean up temp files
    pub fn cleanup(paths: &[&str]) {
        for path in paths {
            let _ = std::fs::remove_file(path);
        }
    }

    fn resolve_script_path() -> Result<PathBuf> {
        Self::resolve_script_path_from(
            std::env::var("PHRON_SCRIPTS_DIR").ok(),
            std::env::current_exe().ok(),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        )
    }

    fn resolve_script_path_from(
        scripts_dir: Option<String>,
        exe: Option<PathBuf>,
        manifest_dir: PathBuf,
    ) -> Result<PathBuf> {
        // 1. Explicit env var — set this in LaunchAgent or shell for installed binary
        if let Some(dir) = scripts_dir {
            let p = PathBuf::from(dir).join("audio_analysis.py");
            if p.exists() {
                return Ok(p);
            }
        }

        // 2. Walk up from exe — works for target/release/comes-bot (dev)
        if let Some(exe) = exe {
            for depth in 1..=4 {
                let mut candidate = exe.clone();
                for _ in 0..depth {
                    candidate.pop();
                }
                let p = candidate.join("scripts").join("audio_analysis.py");
                if p.exists() {
                    return Ok(p);
                }
            }
        }

        // 3. Hardcoded repo path fallback
        let fallback = manifest_dir.join("scripts").join("audio_analysis.py");
        if fallback.exists() {
            return Ok(fallback);
        }

        anyhow::bail!(
            "audio_analysis.py not found. Set PHRON_SCRIPTS_DIR=~/code/phron/scripts in your environment."
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sample_features_json() -> &'static str {
        r#"{
            "duration_seconds": 12.5,
            "wpm_estimate": 140,
            "pause_count": 3,
            "pause_ratio": 0.2,
            "pitch_mean_hz": 180.0,
            "pitch_std_hz": 15.5,
            "pitch_variation": "moderate"
        }"#
    }

    #[test]
    fn parse_audio_features_happy_path() {
        let features =
            AudioProcessor::parse_audio_features(sample_features_json().as_bytes()).unwrap();
        assert_eq!(features.duration_seconds, 12.5);
        assert_eq!(features.wpm_estimate, 140);
        assert_eq!(features.pause_count, 3);
        assert_eq!(features.pause_ratio, 0.2);
        assert_eq!(features.pitch_mean_hz, 180.0);
        assert_eq!(features.pitch_std_hz, 15.5);
        assert_eq!(features.pitch_variation, "moderate");
    }

    #[test]
    fn parse_audio_features_extra_fields_ignored() {
        let json = r#"{"duration_seconds": 1.0, "wpm_estimate": 1, "pause_count": 0, "pause_ratio": 0.0, "pitch_mean_hz": 0.0, "pitch_std_hz": 0.0, "pitch_variation": "low", "extra": true}"#;
        AudioProcessor::parse_audio_features(json.as_bytes()).unwrap();
    }

    #[test]
    fn parse_audio_features_malformed_json_errors() {
        assert!(AudioProcessor::parse_audio_features(b"not-json").is_err());
    }

    #[test]
    fn parse_audio_features_missing_field_errors() {
        let json = r#"{"duration_seconds": 1.0, "wpm_estimate": 1}"#;
        assert!(AudioProcessor::parse_audio_features(json.as_bytes()).is_err());
    }

    #[test]
    fn parse_audio_features_wrong_type_errors() {
        let json = r#"{"duration_seconds": "long", "wpm_estimate": 1, "pause_count": 0, "pause_ratio": 0.0, "pitch_mean_hz": 0.0, "pitch_std_hz": 0.0, "pitch_variation": "low"}"#;
        assert!(AudioProcessor::parse_audio_features(json.as_bytes()).is_err());
    }

    #[test]
    fn cleanup_deletes_existing_and_ignores_missing() {
        let dir = crate::test_support::temp_dir();
        let keep_missing = dir.join("gone.wav");
        let existing = dir.join("temp.wav");
        fs::write(&existing, b"wav").unwrap();
        AudioProcessor::cleanup(&[existing.to_str().unwrap(), keep_missing.to_str().unwrap()]);
        assert!(!existing.exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cleanup_empty_slice_is_ok() {
        AudioProcessor::cleanup(&[]);
    }

    #[test]
    fn resolve_prefers_scripts_dir_when_file_exists() {
        let dir = crate::test_support::temp_dir();
        let script = dir.join("audio_analysis.py");
        fs::write(&script, "# test").unwrap();
        let resolved = AudioProcessor::resolve_script_path_from(
            Some(dir.to_string_lossy().into_owned()),
            None,
            PathBuf::from("/does/not/exist"),
        )
        .unwrap();
        assert_eq!(resolved, script);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resolve_walks_up_from_exe() {
        let root = crate::test_support::temp_dir();
        let scripts = root.join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        let script = scripts.join("audio_analysis.py");
        fs::write(&script, "# test").unwrap();
        let exe = root.join("target").join("release").join("comes-bot");
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        fs::write(&exe, b"").unwrap();

        let resolved = AudioProcessor::resolve_script_path_from(
            None,
            Some(exe),
            PathBuf::from("/does/not/exist"),
        )
        .unwrap();
        assert_eq!(resolved, script);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolve_falls_back_to_manifest_dir() {
        let root = crate::test_support::temp_dir();
        let scripts = root.join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        let script = scripts.join("audio_analysis.py");
        fs::write(&script, "# test").unwrap();

        let resolved = AudioProcessor::resolve_script_path_from(None, None, root.clone()).unwrap();
        assert_eq!(resolved, script);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolve_errors_when_nothing_exists() {
        let err = AudioProcessor::resolve_script_path_from(
            Some("/no/such/scripts/dir".into()),
            Some(PathBuf::from("/tmp/not-an-exe")),
            PathBuf::from("/also/missing"),
        )
        .unwrap_err();
        assert!(err.to_string().contains("audio_analysis.py not found"));
    }

    #[test]
    fn resolve_skips_scripts_dir_when_file_missing() {
        let missing_env = crate::test_support::temp_dir();
        let fallback_root = crate::test_support::temp_dir();
        fs::create_dir_all(fallback_root.join("scripts")).unwrap();
        let script = fallback_root.join("scripts").join("audio_analysis.py");
        fs::write(&script, "# test").unwrap();

        let resolved = AudioProcessor::resolve_script_path_from(
            Some(missing_env.to_string_lossy().into_owned()),
            None,
            fallback_root.clone(),
        )
        .unwrap();
        assert_eq!(resolved, script);
        let _ = fs::remove_dir_all(missing_env);
        let _ = fs::remove_dir_all(fallback_root);
    }
}
