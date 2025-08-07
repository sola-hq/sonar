use std::sync::atomic::{AtomicU64, Ordering};
use tracing::info;

#[derive(Debug, Default)]
pub struct NodeMetrics {
    pub total_swaps_processed: AtomicU64,
    pub succeed_swaps: AtomicU64,
    pub failed_swaps: AtomicU64,
    pub skipped_tiny_swaps: AtomicU64,
    pub skipped_zero_swaps: AtomicU64,
    pub skipped_no_metadata: AtomicU64,
    pub skipped_unexpected_swaps: AtomicU64,
    pub skipped_unknown_swaps: AtomicU64,
    pub message_send_success: AtomicU64,
    pub message_send_failure: AtomicU64,
    pub db_insert_success: AtomicU64,
    pub db_insert_failure: AtomicU64,
    pub kv_insert_success: AtomicU64,
    pub kv_insert_failure: AtomicU64,
}

impl NodeMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment_total_swaps(&self) {
        let count = self.total_swaps_processed.fetch_add(1, Ordering::Relaxed);
        if (count + 1) % 5000 == 0 {
            self.log_metrics();
        }
    }

    pub fn increment_succeed_swaps(&self) {
        self.succeed_swaps.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_failed_swaps(&self) {
        self.failed_swaps.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_skipped_tiny_swaps(&self) {
        self.skipped_tiny_swaps.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_skipped_zero_swaps(&self) {
        self.skipped_zero_swaps.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_skipped_no_metadata(&self) {
        self.skipped_no_metadata.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_skipped_unexpected_swaps(&self) {
        self.skipped_unexpected_swaps.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_skipped_unknown_swaps(&self) {
        self.skipped_unknown_swaps.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_db_insert_success(&self) {
        self.db_insert_success.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_db_insert_failure(&self) {
        self.db_insert_failure.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_message_send_success(&self) {
        self.message_send_success.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_message_send_failure(&self) {
        self.message_send_failure.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_kv_insert_success(&self) {
        self.kv_insert_success.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_kv_insert_failure(&self) {
        self.kv_insert_failure.fetch_add(1, Ordering::Relaxed);
    }

    fn log_metrics(&self) {
        let total = self.total_swaps_processed.load(Ordering::Relaxed);
        let succeed = self.succeed_swaps.load(Ordering::Relaxed);
        let failed = self.failed_swaps.load(Ordering::Relaxed);
        let tiny = self.skipped_tiny_swaps.load(Ordering::Relaxed);
        let zero = self.skipped_zero_swaps.load(Ordering::Relaxed);
        let unexpected = self.skipped_unexpected_swaps.load(Ordering::Relaxed);
        let unknown = self.skipped_unknown_swaps.load(Ordering::Relaxed);
        let message_send_success = self.message_send_success.load(Ordering::Relaxed);
        let message_send_failure = self.message_send_failure.load(Ordering::Relaxed);
        let db_insert_success = self.db_insert_success.load(Ordering::Relaxed);
        let db_insert_failure = self.db_insert_failure.load(Ordering::Relaxed);
        let kv_insert_success = self.kv_insert_success.load(Ordering::Relaxed);
        let kv_insert_failure = self.kv_insert_failure.load(Ordering::Relaxed);

        let success_rate = if total > 0 { (succeed as f64 / total as f64) * 100.0 } else { 0.0 };

        info!(
            total_processed = total,
            succeed = succeed,
            success_rate = format!("{:.1}%", success_rate),
            failed = failed,
            skipped_tiny_swaps = tiny,
            skipped_zero_swaps = zero,
            skipped_unexpected_swaps = unexpected,
            skipped_unknown_swaps = unknown,
            message_send_success = message_send_success,
            message_send_failure = message_send_failure,
            db_insert_success = db_insert_success,
            db_insert_failure = db_insert_failure,
            kv_insert_success = kv_insert_success,
            kv_insert_failure = kv_insert_failure,
            "swap_metrics"
        );
    }
}
