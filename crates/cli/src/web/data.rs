//! Ce que les onglets Modes, Historique et Tableau de bord lisent : des vues JSON des données du cœur (modes, historique, chiffres).

use chrono::{DateTime, TimeZone, Utc};
use scriibe_core::history::{Calendar, Dictation, Store};
use scriibe_core::modes::RewriteMode;
use scriibe_core::settings::AppSettings;
use serde_json::{json, Value};

/// Les huit modes livrés, avec leur phrase de style d'origine et celle qui est réglée.
pub fn modes(settings: &AppSettings) -> Value {
    let built_in: Vec<Value> = RewriteMode::ALL
        .iter()
        .map(|mode| {
            json!({
                "id": mode.raw(),
                "label": mode.label(),
                "summary": mode.summary(),
                "has_style": mode.has_style(),
                "original_style": mode.style(),
                "typed_style": settings.typed_style(*mode),
                "modified": settings.is_style_modified(*mode),
                // Le moteur schtroumpf n'est pas encore porté : Amusant se comporte comme Simple (docs/ECARTS.md).
                "available": *mode != RewriteMode::Playful,
            })
        })
        .collect();
    json!({
        "built_in": built_in,
        "groups": RewriteMode::MENU_GROUPS.iter().map(|g| g.iter().map(|m| m.raw()).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "choice": settings.mode_choice(),
        "active_label": settings.active_mode_label(),
    })
}

fn dictation(d: &Dictation) -> Value {
    json!({
        "id": d.id,
        "date": d.date.to_rfc3339(),
        "app": d.app_name,
        "raw": d.raw_text,
        "text": d.final_text,
        "has_text": d.has_text(),
        "words": d.words,
        "duration": d.duration,
        "wpm": d.words_per_minute(),
        "corrections": d.corrections,
        "translation": d.translation.map(|t| t.name()),
        "timing": d.timing,
    })
}

/// L'historique, le plus récent d'abord.
pub fn history(store: &Store) -> Value {
    json!({"items": store.history.iter().map(dictation).collect::<Vec<_>>(), "apps": store.app_names()})
}

/// Les chiffres du tableau de bord (archive des jours purgés comprise, comme dans Scribe).
pub fn stats<Tz: TimeZone>(store: &Store, calendar: &Calendar<Tz>, now: DateTime<Utc>) -> Value {
    json!({
        "dictations": store.dictation_count(),
        "total_words": store.total_words(),
        "today_words": store.today_words(calendar, now),
        "yesterday_words": store.yesterday_words(calendar, now),
        "today_change": store.today_change(calendar, now),
        "average_wpm": store.average_wpm(),
        "time_saved": store.time_saved(),
        "streak": store.streak(calendar, now),
        "corrections_this_week": store.corrections_this_week(calendar, now),
        "last_days": store.last_days(calendar, now).iter().map(|(day, words)| json!({"day": day.to_rfc3339(), "words": words})).collect::<Vec<_>>(),
        "top_apps": store.top_apps().iter().map(|a| json!({"name": a.name, "words": a.words, "share": a.share})).collect::<Vec<_>>(),
        "typing_wpm": store.settings.typing_wpm,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn store() -> (Store, DateTime<Utc>) {
        let now = Utc.with_ymd_and_hms(2026, 10, 27, 12, 0, 0).unwrap();
        let mut store = Store::default();
        for (days_ago, app, words, seconds) in [(0, "Bloc-notes", 30, 12.0), (0, "Chrome", 10, 5.0), (1, "Bloc-notes", 50, 20.0)] {
            store.history.push(Dictation::new(now - Duration::days(days_ago), app, "brut", "propre", words, seconds, 1));
        }
        (store, now)
    }

    #[test]
    fn stats_count_words_days_and_apps() {
        let (store, now) = store();
        let stats = stats(&store, &Calendar::new(Utc), now);
        assert_eq!(stats["dictations"], 3);
        assert_eq!(stats["total_words"], 90);
        assert_eq!(stats["today_words"], 40);
        assert_eq!(stats["yesterday_words"], 50);
        assert_eq!(stats["streak"], 2);
        assert_eq!(stats["corrections_this_week"], 3);
        assert_eq!(stats["last_days"].as_array().unwrap().len(), 14);
        assert_eq!(stats["top_apps"][0]["name"], "Bloc-notes");
        assert_eq!(stats["top_apps"][0]["words"], 80);
    }

    #[test]
    fn history_lists_the_dictations_with_their_text() {
        let (store, _) = store();
        let view = history(&store);
        assert_eq!(view["items"].as_array().unwrap().len(), 3);
        assert_eq!(view["items"][0]["text"], "propre");
        assert_eq!(view["apps"], json!(["Bloc-notes", "Chrome"]));
    }

    #[test]
    fn modes_lists_the_eight_built_in_modes_and_marks_the_playful_one_unavailable() {
        let mut settings = AppSettings::default();
        let view = modes(&settings);
        assert_eq!(view["built_in"].as_array().unwrap().len(), 8);
        assert_eq!(view["choice"], "simple");
        assert_eq!(view["built_in"][7]["available"], false);
        settings.style_overrides.insert("formal".into(), "Écris comme un notaire".into());
        let view = modes(&settings);
        assert_eq!(view["built_in"][2]["modified"], true);
        assert_eq!(view["built_in"][2]["typed_style"], "Écris comme un notaire");
    }
}
