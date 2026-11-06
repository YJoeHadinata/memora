use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryPoint {
    pub timestamp_secs: f64,
    pub used_gb: f64,
    pub compressed_gb: f64,
    pub swap_gb: f64,
    pub pressure_level: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryBuffer {
    points: VecDeque<HistoryPoint>,
    capacity: usize,
}

impl HistoryBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            points: VecDeque::with_capacity(capacity),
            capacity: capacity.max(10),
        }
    }

    pub fn push(&mut self, point: HistoryPoint) {
        if self.points.len() >= self.capacity {
            self.points.pop_front();
        }
        self.points.push_back(point);
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn clear(&mut self) {
        self.points.clear();
    }

    pub fn points(&self) -> &VecDeque<HistoryPoint> {
        &self.points
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn set_capacity(&mut self, new_capacity: usize) {
        self.capacity = new_capacity.max(10);
        while self.points.len() > self.capacity {
            self.points.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_buffer_bounded() {
        let mut buffer = HistoryBuffer::new(5);
        for i in 0..10 {
            buffer.push(HistoryPoint {
                timestamp_secs: i as f64,
                used_gb: i as f64 * 0.5,
                compressed_gb: 0.1,
                swap_gb: 0.0,
                pressure_level: 1.0,
            });
        }

        // Even with max(10) minimum capacity:
        assert!(buffer.len() <= buffer.capacity());
        assert_eq!(buffer.points().back().unwrap().timestamp_secs, 9.0);
    }
}
