use std::time::Instant;

use chrono::{DateTime, Utc};

use crate::core::{
    ports::Ports,
    scanner::{self, ScanResult},
    target::Target,
};

/// Engine — scanner ustidagi yupqa qatlam.
/// Vazifasi:
/// - target yaratish
/// - scanner ishga tushirish
/// - natijani o‘zgartirmasdan qaytarish
///
/// ⚠️ v0.1.x API — SAQLANADI
pub fn run(target_input: &str, ports: Ports) -> Vec<ScanResult> {
    let target = Target::new(target_input);

    scanner::scan(&target, &ports)
}

// =======================
// v0.2.0 YANGI QATLAM
// =======================

/// Scan haqida meta ma’lumotlar (v0.2.0)
#[derive(Debug, Clone)]
pub struct ScanMeta {
    pub target: String,
    pub started_at: DateTime<Utc>,
    pub duration_ms: u128,
}

/// Kengaytirilgan run — meta bilan
///
/// ❗️Eski run() ni almashtirmaydi
/// ❗️Terminal va export bunga asta-sekin o‘tkaziladi
pub fn run_with_meta(
    target_input: &str,
    ports: Ports,
) -> (Vec<ScanResult>, ScanMeta) {
    let started_at = Utc::now();
    let timer = Instant::now();

    let target = Target::new(target_input);
    let results = scanner::scan(&target, &ports);

    let meta = ScanMeta {
        target: target_input.to_string(),
        started_at,
        duration_ms: timer.elapsed().as_millis(),
    };

    (results, meta)
}
