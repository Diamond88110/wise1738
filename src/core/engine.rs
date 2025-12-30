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
pub fn run(target_input: &str, ports: Ports) -> Vec<ScanResult> {
    let target = Target::new(target_input);

    // Scanner allaqachon:
    // - parallel
    // - OPEN / CLOSED / FILTERED
    // - service nomi bilan qaytaradi
    scanner::scan(&target, &ports)
}
