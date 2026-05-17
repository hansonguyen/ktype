#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionResult {
    pub timestamp: i64,
    pub duration_secs: u64,
    pub wpm: f64,
    pub raw_wpm: f64,
    pub accuracy: f64,
}
