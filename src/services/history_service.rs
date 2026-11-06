use std::time::SystemTime;

use crate::models::{HistoryBuffer, HistoryPoint, MemorySnapshot};

pub struct HistoryService {
    buffer: HistoryBuffer,
    start_time: SystemTime,
    peak_used_bytes: u64,
    min_used_bytes: u64,
    samples_count: u64,
    total_used_sum: u64,
}

impl HistoryService {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: HistoryBuffer::new(capacity),
            start_time: SystemTime::now(),
            peak_used_bytes: 0,
            min_used_bytes: u64::MAX,
            samples_count: 0,
            total_used_sum: 0,
        }
    }

    pub fn record(&mut self, snapshot: &MemorySnapshot) {
        let elapsed = snapshot
            .timestamp
            .duration_since(self.start_time)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);

        let to_gb = |bytes: u64| bytes as f64 / (1024.0 * 1024.0 * 1024.0);

        let point = HistoryPoint {
            timestamp_secs: elapsed,
            used_gb: to_gb(snapshot.used_bytes),
            compressed_gb: to_gb(snapshot.compressed_bytes),
            swap_gb: to_gb(snapshot.swap_used_bytes),
            pressure_level: snapshot.pressure.level_numeric(),
        };

        self.buffer.push(point);

        // Update session stats
        self.peak_used_bytes = self.peak_used_bytes.max(snapshot.used_bytes);
        self.min_used_bytes = self.min_used_bytes.min(snapshot.used_bytes);
        self.samples_count = self.samples_count.saturating_add(1);
        self.total_used_sum = self.total_used_sum.saturating_add(snapshot.used_bytes);
    }

    pub fn peak_used_bytes(&self) -> u64 {
        self.peak_used_bytes
    }

    pub fn min_used_bytes(&self) -> u64 {
        if self.min_used_bytes == u64::MAX {
            0
        } else {
            self.min_used_bytes
        }
    }

    pub fn average_used_bytes(&self) -> u64 {
        self.total_used_sum
            .checked_div(self.samples_count)
            .unwrap_or(0)
    }

    pub fn buffer(&self) -> &HistoryBuffer {
        &self.buffer
    }

    pub fn buffer_mut(&mut self) -> &mut HistoryBuffer {
        &mut self.buffer
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.start_time = SystemTime::now();
        self.peak_used_bytes = 0;
        self.min_used_bytes = u64::MAX;
        self.samples_count = 0;
        self.total_used_sum = 0;
    }
}
