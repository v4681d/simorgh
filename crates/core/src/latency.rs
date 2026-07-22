//! Per-stage latency instrumentation. Every AudioFrame carries `captured_at`;
//! stages record spans against it so the UI can show honest end-to-end and
//! per-stage numbers (never fake lower values).

use crate::types::Stage;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

const WINDOW: usize = 50;

#[derive(Default)]
pub struct LatencyRecorder {
    samples: Mutex<HashMap<Stage, Vec<Duration>>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LatencyReport {
    /// Rolling average per stage, milliseconds.
    pub per_stage_ms: HashMap<Stage, u64>,
    /// Sum of stage averages — the honest end-to-end estimate, milliseconds.
    pub end_to_end_ms: u64,
}

impl LatencyRecorder {
    pub fn record(&self, stage: Stage, elapsed: Duration) {
        let mut map = self.samples.lock().expect("latency lock");
        let v = map.entry(stage).or_default();
        v.push(elapsed);
        if v.len() > WINDOW {
            let drop_n = v.len() - WINDOW;
            v.drain(..drop_n);
        }
    }

    pub fn report(&self) -> LatencyReport {
        let map = self.samples.lock().expect("latency lock");
        let mut per_stage_ms = HashMap::new();
        let mut total = 0u64;
        for (stage, v) in map.iter() {
            if v.is_empty() { continue; }
            let avg = v.iter().map(|d| d.as_millis() as u64).sum::<u64>() / v.len() as u64;
            per_stage_ms.insert(*stage, avg);
            total += avg;
        }
        LatencyReport { per_stage_ms, end_to_end_ms: total }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolling_average() {
        let r = LatencyRecorder::default();
        r.record(Stage::Stt, Duration::from_millis(100));
        r.record(Stage::Stt, Duration::from_millis(300));
        r.record(Stage::Translate, Duration::from_millis(50));
        let rep = r.report();
        assert_eq!(rep.per_stage_ms[&Stage::Stt], 200);
        assert_eq!(rep.end_to_end_ms, 250);
    }
}
