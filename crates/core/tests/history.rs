#![allow(clippy::field_reassign_with_default)] // les tests partent des réglages par défaut et en changent un ou deux champs, comme Scribe
//! Scénarios de conservation de l'historique (cas `history_retention-003` à `-013` de `fixtures/history_retention.json`) :
//! le texte part, les totaux par jour restent.

use chrono::{DateTime, Duration, TimeZone, Utc};
use scriibe_core::history::{retention_calc, Calendar, Dictation, Store};

fn cal() -> Calendar<Utc> {
    Calendar::new(Utc)
}

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 22, 15, 0, 0).unwrap()
}

/// 10 jours (0 = aujourd'hui) × 2 dictées par jour, à midi ; la 2e est 90 min avant la 1re.
/// k = 0 : « Mail », 20 mots ; k = 1 : « Notes », 30 mots ; durée = mots / 130 × 60 s ; 1 correction.
fn store_with_ten_days() -> Store {
    let c = cal();
    let noon = c.start_of_day(now()) + Duration::hours(12);
    let mut store = Store::default();
    for offset in 0..10 {
        for k in 0..2 {
            let date = c.add_days(noon, -offset) - Duration::minutes(k * 90);
            let words = 20 + k * 10;
            store.history.push(Dictation::new(
                date,
                if k == 0 { "Mail" } else { "Notes" },
                &format!("brut {offset}"),
                &format!("texte {offset} {k}"),
                words,
                words as f64 / 130.0 * 60.0,
                1,
            ));
        }
    }
    store
}

/// history_retention-003 : l'aperçu annonce ce qui est supprimé, le reste est gardé.
#[test]
fn history_retention_003_preview_matches_the_purge() {
    let mut store = store_with_ten_days();
    let planned = store.preview_history_change(&cal(), now(), true, Some(3));
    let purged = store.purge_history(&cal(), now(), 3);
    assert_eq!((planned.purged, planned.erased), (14, 0));
    assert_eq!(purged, 14);
    assert_eq!(store.history.len(), 6);
}

/// history_retention-004 : les chiffres du tableau de bord ne bougent pas.
#[test]
fn history_retention_004_dashboard_numbers_do_not_move() {
    let mut store = store_with_ten_days();
    let c = cal();
    let eight_days_ago = c.add_days(now(), -8);
    let snapshot = |s: &Store| {
        (
            s.total_words(),
            s.dictation_count(),
            s.average_wpm(),
            s.time_saved(),
            s.streak(&c, now()),
            s.top_apps().into_iter().map(|a| a.name).collect::<Vec<_>>(),
            s.corrections_this_week(&c, now()),
            s.words_on(&c, eight_days_ago),
        )
    };
    let before = snapshot(&store);
    store.purge_history(&c, now(), 3);
    let after = snapshot(&store);
    assert_eq!((before.0, before.1, before.2), (after.0, after.1, after.2));
    assert!((before.3 - after.3).abs() < 0.001);
    assert_eq!((before.4, &before.5, before.6, before.7), (after.4, &after.5, after.6, after.7));
    assert_eq!(before.7, 50);
}

/// history_retention-005 : les totaux par jour n'ont aucun texte et s'additionnent.
#[test]
fn history_retention_005_daily_totals_add_up() {
    let mut store = store_with_ten_days();
    store.purge_history(&cal(), now(), 3);
    assert_eq!(store.archive.len(), 7);
    assert_eq!(store.archive.iter().map(|d| d.dictations).sum::<i64>(), 14);
    assert!(store.archive.iter().all(|d| d.apps.get("Mail") == Some(&20) && d.apps.get("Notes") == Some(&30)));
    // Deux totaux du même jour s'additionnent : 20 + 30 mots, deux fois.
    let doubled = retention_calc::merged(&store.archive, &store.archive);
    assert_eq!(doubled.len(), 7);
    assert_eq!(doubled.first().map(|d| d.words), Some(100));
}

/// history_retention-006 : un second passage ne supprime plus rien.
#[test]
fn history_retention_006_second_purge_removes_nothing() {
    let mut store = store_with_ten_days();
    store.purge_history(&cal(), now(), 3);
    assert_eq!(store.purge_history(&cal(), now(), 3), 0);
}

/// history_retention-007 : texte non gardé, la fiche chiffrée reste.
#[test]
fn history_retention_007_text_not_kept() {
    let mut store = Store::default();
    store.settings.keep_dictation_text = false;
    store.add(&cal(), now(), Dictation::new(now(), "Mail", "secret brut", "secret", 1, 1.0, 0));
    let first = &store.history[0];
    assert_eq!((first.final_text.as_str(), first.raw_text.as_str(), first.words, first.has_text()), ("", "", 1, false));
    assert_eq!(store.last_dictation_text.as_deref(), Some("secret"), "« copier la dernière » marche encore");
}

/// history_retention-008 : effacer les textes existants.
#[test]
fn history_retention_008_erase_existing_texts() {
    let mut store = store_with_ten_days();
    let c = cal();
    store.purge_history(&c, now(), 3);
    store.settings.keep_dictation_text = true;
    store.last_dictation_text = Some("secret".into());
    let words_before = store.total_words();
    let plan = store.preview_history_change(&c, now(), false, None);
    store.apply_history_change(&c, now(), false, Some(2));
    assert_eq!(plan.erased, 6);
    assert!(store.history.iter().all(|d| !d.has_text()));
    assert_eq!(store.total_words(), words_before);
    assert_eq!(store.settings.history_keep_days, Some(2));
    assert!(!store.settings.keep_dictation_text);
    assert_eq!(store.last_dictation_text, None);
}

/// history_retention-009 : tout effacer vide aussi les totaux par jour.
#[test]
fn history_retention_009_clear_everything() {
    let mut store = store_with_ten_days();
    store.purge_history(&cal(), now(), 3);
    store.clear_history();
    assert!(store.history.is_empty() && store.archive.is_empty());
    assert_eq!(store.total_words(), 0);
}

/// history_retention-010 : le temps gagné d'un jour purgé reste proche du calcul dictée par dictée.
#[test]
fn history_retention_010_time_saved_per_day() {
    let c = cal();
    let items = vec![Dictation::new(now(), "Mail", "", "", 3, 10.0, 0), Dictation::new(now(), "Mail", "", "", 60, 10.0, 0)];
    let mut per_dictation = Store::default();
    per_dictation.history = items.clone();
    let mut per_day = Store::default();
    per_day.archive = retention_calc::totals(&c, &items);
    let (a, b) = (per_dictation.time_saved(), per_day.time_saved());
    assert!(a > b && a - b < 0.1 * a, "{a} contre {b}");
    assert!((a - 80.0).abs() < 0.01 && (b - 74.5).abs() < 0.01, "80 s contre 74,5 s dans le test de Scribe");
}

/// history_retention-011 : la dernière dictée supprimée n'est plus copiable.
#[test]
fn history_retention_011_deleted_last_dictation() {
    let mut store = Store::default();
    store.add(&cal(), now(), Dictation::new(now(), "Mail", "a", "a", 1, 1.0, 0));
    let id = store.history[0].id.clone();
    store.delete(&id);
    assert_eq!(store.last_dictation_text, None);
}

/// history_retention-012 : la durée de conservation s'applique seule, une fois par jour.
#[test]
fn history_retention_012_retention_applies_by_itself() {
    let mut store = store_with_ten_days();
    let c = cal();
    store.apply_history_change(&c, now(), false, Some(2));
    let cutoff = retention_calc::cutoff(&c, 2, now());
    assert!(store.history.iter().all(|d| d.date >= cutoff));
    assert!(store.retention_day.is_some());
}

/// history_retention-013 : la durée de conservation ne repasse pas deux fois le même jour.
#[test]
fn history_retention_013_retention_runs_once_a_day() {
    let mut store = store_with_ten_days();
    let c = cal();
    store.apply_history_change(&c, now(), false, Some(2));
    store.add(&c, now(), Dictation::new(now() - Duration::days(30), "Mail", "", "", 5, 2.0, 0));
    assert!(store.history.iter().any(|d| d.words == 5));
}
