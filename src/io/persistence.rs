use std::path::{Path, PathBuf};

use crate::io::stats::SessionResult;

#[derive(Debug, thiserror::Error)]
pub enum PersistError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

fn stats_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("ktype")
        .join("stats.json")
}

pub fn load() -> Result<Vec<SessionResult>, PersistError> {
    load_from(&stats_path())
}

pub fn append(result: &SessionResult) -> Result<(), PersistError> {
    append_to(&stats_path(), result)
}

pub(crate) fn load_from(path: &Path) -> Result<Vec<SessionResult>, PersistError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&data)?)
}

pub(crate) fn append_to(path: &Path, result: &SessionResult) -> Result<(), PersistError> {
    let mut entries = if path.exists() {
        let data = std::fs::read_to_string(path)?;
        serde_json::from_str::<Vec<SessionResult>>(&data)?
    } else {
        Vec::new()
    };
    entries.push(result.clone());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&entries)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use tempfile::TempDir;

    fn sample_result() -> SessionResult {
        SessionResult {
            timestamp: 1_000_000,
            duration_secs: 15,
            wpm: 60.0,
            raw_wpm: 65.0,
            accuracy: 92.0,
        }
    }

    fn arb_result() -> impl Strategy<Value = SessionResult> {
        (
            any::<i64>(),
            any::<u64>(),
            0.0f64..200.0f64,
            0.0f64..200.0f64,
            0.0f64..100.0f64,
        )
            .prop_map(
                |(timestamp, duration_secs, wpm, raw_wpm, accuracy)| SessionResult {
                    timestamp,
                    duration_secs,
                    wpm,
                    raw_wpm,
                    accuracy,
                },
            )
    }

    #[test]
    fn load_missing_file_returns_empty_vec() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("stats.json");
        let result = load_from(&path).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn append_creates_file_and_loads_back() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("stats.json");
        let r = sample_result();
        append_to(&path, &r).unwrap();
        let loaded = load_from(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].timestamp, r.timestamp);
        assert!((loaded[0].wpm - r.wpm).abs() < 0.01);
    }

    #[test]
    fn append_accumulates_multiple_results() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("stats.json");
        append_to(&path, &sample_result()).unwrap();
        append_to(&path, &sample_result()).unwrap();
        let loaded = load_from(&path).unwrap();
        assert_eq!(loaded.len(), 2);
    }

    proptest! {
        #[test]
        fn append_n_load_n_round_trip(results in prop::collection::vec(arb_result(), 1..=20)) {
            let dir = TempDir::new().unwrap();
            let path = dir.path().join("stats.json");

            for result in &results {
                append_to(&path, result).unwrap();
            }

            let loaded = load_from(&path).unwrap();
            prop_assert_eq!(loaded.len(), results.len());

            for (orig, loaded_entry) in results.iter().zip(loaded.iter()) {
                prop_assert_eq!(orig.timestamp, loaded_entry.timestamp);
                prop_assert_eq!(orig.duration_secs, loaded_entry.duration_secs);
                prop_assert!((orig.wpm - loaded_entry.wpm).abs() < 1e-9);
                prop_assert!((orig.raw_wpm - loaded_entry.raw_wpm).abs() < 1e-9);
                prop_assert!((orig.accuracy - loaded_entry.accuracy).abs() < 1e-9);
            }
        }
    }
}
