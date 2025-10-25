use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Performance metrics for UTP operations
pub struct UtpMetrics {
    // Latency tracking
    traditional_latencies: Mutex<Vec<u64>>,
    utp_latencies: Mutex<Vec<u64>>,

    // Size tracking
    traditional_sizes: Mutex<Vec<usize>>,
    utp_sizes: Mutex<Vec<usize>>,

    // Request counter
    pub total_requests: AtomicU64,
}

impl UtpMetrics {
    pub fn new() -> Self {
        Self {
            traditional_latencies: Mutex::new(Vec::new()),
            utp_latencies: Mutex::new(Vec::new()),
            traditional_sizes: Mutex::new(Vec::new()),
            utp_sizes: Mutex::new(Vec::new()),
            total_requests: AtomicU64::new(0),
        }
    }

    pub fn record_traditional(&self, latency_us: u64, size_bytes: usize) {
        if let Ok(mut latencies) = self.traditional_latencies.lock() {
            latencies.push(latency_us);
        }
        if let Ok(mut sizes) = self.traditional_sizes.lock() {
            sizes.push(size_bytes);
        }
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_utp(&self, latency_us: u64, size_bytes: usize) {
        if let Ok(mut latencies) = self.utp_latencies.lock() {
            latencies.push(latency_us);
        }
        if let Ok(mut sizes) = self.utp_sizes.lock() {
            sizes.push(size_bytes);
        }
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> MetricsSnapshot {
        let trad_latencies = self.traditional_latencies.lock().unwrap().clone();
        let utp_latencies = self.utp_latencies.lock().unwrap().clone();
        let trad_sizes = self.traditional_sizes.lock().unwrap().clone();
        let utp_sizes = self.utp_sizes.lock().unwrap().clone();

        let avg_traditional_latency = Self::average(&trad_latencies);
        let avg_utp_latency = Self::average(&utp_latencies);
        let avg_traditional_size = Self::average_usize(&trad_sizes);
        let avg_utp_size = Self::average_usize(&utp_sizes);

        let speedup_factor = if avg_utp_latency > 0 {
            avg_traditional_latency as f64 / avg_utp_latency as f64
        } else {
            1.0
        };

        let size_reduction = if avg_traditional_size > 0 {
            ((avg_traditional_size as f64 - avg_utp_size as f64) / avg_traditional_size as f64) * 100.0
        } else {
            0.0
        };

        MetricsSnapshot {
            avg_latency_traditional_us: avg_traditional_latency,
            avg_latency_utp_us: avg_utp_latency,
            speedup_factor,
            avg_size_traditional_bytes: avg_traditional_size,
            avg_size_utp_bytes: avg_utp_size,
            size_reduction_percent: size_reduction,
            total_requests: self.total_requests.load(Ordering::Relaxed),
        }
    }

    fn average(values: &[u64]) -> u64 {
        if values.is_empty() {
            return 0;
        }
        let sum: u64 = values.iter().sum();
        sum / values.len() as u64
    }

    fn average_usize(values: &[usize]) -> usize {
        if values.is_empty() {
            return 0;
        }
        let sum: usize = values.iter().sum();
        sum / values.len()
    }
}

impl Default for UtpMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub avg_latency_traditional_us: u64,
    pub avg_latency_utp_us: u64,
    pub speedup_factor: f64,
    pub avg_size_traditional_bytes: usize,
    pub avg_size_utp_bytes: usize,
    pub size_reduction_percent: f64,
    pub total_requests: u64,
}
