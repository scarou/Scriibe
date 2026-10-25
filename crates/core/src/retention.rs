//! Durée de conservation de l'historique : bornes (port de `HistoryRetention.swift`, partie sans calendrier).

pub const MINIMUM_DAYS: i64 = 2;
pub const MAXIMUM_DAYS: i64 = 3650;

/// Ramène une durée dans les bornes permises.
pub fn clamped(days: i64) -> i64 {
    days.clamp(MINIMUM_DAYS, MAXIMUM_DAYS)
}
