# /// script
# requires-python = ">=3.11"
# dependencies = ["librosa", "numpy"]
# ///

import sys
import json
import librosa
import numpy as np

def analyze_audio(file_path):
    try:
        # Load audio file (convert to mono, 22050Hz default)
        y, sr = librosa.load(file_path, sr=None)
        duration = librosa.get_duration(y=y, sr=sr)

        # Pause detection: segments with silence > 0.3s
        # librosa.effects.split returns intervals of non-silent audio
        non_silent_intervals = librosa.effects.split(y, top_db=30)
        
        pause_count = len(non_silent_intervals) - 1 if len(non_silent_intervals) > 0 else 0
        
        non_silent_duration = sum([(end - start) / sr for start, end in non_silent_intervals])
        pause_ratio = max(0.0, (duration - non_silent_duration) / duration) if duration > 0 else 0.0

        # WPM estimate: assume ~130 WPM average for non-silent parts
        # adjust based on silence ratio
        wpm_estimate = int(130 * (non_silent_duration / duration) * (duration / 60)) if duration > 0 else 0
        # Actually, let's just do a simple duration-based estimate as requested
        # WPM = (words / duration_min). If we assume 130 words per minute of speaking time:
        wpm_estimate = int(130 * (non_silent_duration / 60)) if duration > 0 else 0
        
        # Refined WPM: common range is 120-150. If someone speaks for 45s with 15% pauses:
        # 45 * 0.85 = 38.25s of speech. 38.25 / 60 * 140 = 89 words. 89 / (45/60) = 119 WPM.
        wpm_normalized = int((wpm_estimate / (duration / 60))) if duration > 0 else 0

        # Pitch detection using YIN
        f0 = librosa.yin(y, fmin=librosa.note_to_hz('C2'), fmax=librosa.note_to_hz('C7'))
        # Filter out NaN or very low values
        f0 = f0[~np.isnan(f0)]
        f0 = f0[f0 > 0]
        
        if len(f0) > 0:
            pitch_mean = float(np.mean(f0))
            pitch_std = float(np.std(f0))
        else:
            pitch_mean = 0.0
            pitch_std = 0.0

        if pitch_std < 15:
            variation = "monotone"
        elif pitch_std < 40:
            variation = "moderate"
        else:
            variation = "high"

        result = {
            "duration_seconds": round(float(duration), 1),
            "wpm_estimate": wpm_normalized,
            "pause_count": int(pause_count),
            "pause_ratio": round(float(pause_ratio), 2),
            "pitch_mean_hz": round(pitch_mean, 1),
            "pitch_std_hz": round(pitch_std, 1),
            "pitch_variation": variation
        }
        
        return result

    except Exception as e:
        return {"error": str(e)}

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(json.dumps({"error": "No file path provided"}))
        sys.exit(1)
        
    audio_path = sys.argv[1]
    analysis = analyze_audio(audio_path)
    print(json.dumps(analysis))
