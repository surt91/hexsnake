//! Aggregate statistics over self-played (non-autopilot) games.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Stats {
    pub games: u64,
    pub total_final_length: u64,
    pub best_score: u32,
    pub best_length: u32,
}

impl Stats {
    pub fn record(&mut self, score: u32, final_length: u32) {
        self.games += 1;
        self.total_final_length += u64::from(final_length);
        self.best_score = self.best_score.max(score);
        self.best_length = self.best_length.max(final_length);
    }

    pub fn avg_length(&self) -> f64 {
        if self.games == 0 {
            return 0.0;
        }
        self.total_final_length as f64 / self.games as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_averages() {
        let mut stats = Stats::default();
        assert_eq!(stats.avg_length(), 0.0);
        stats.record(5, 8);
        stats.record(11, 14);
        assert_eq!(stats.games, 2);
        assert_eq!(stats.avg_length(), 11.0);
        assert_eq!(stats.best_score, 11);
        assert_eq!(stats.best_length, 14);
    }
}
