//! Historique des dictées, totaux par jour des dictées purgées, conservation et chiffres du tableau de bord.
//! Port de `Store.swift` et `HistoryRetention.swift` (Scribe), sans lecture ni écriture de fichiers : l'état est une structure pure.
//!
//! Le calendrier est passé à chaque appel (fuseau + heure courante) : le cœur ne lit pas l'horloge du système, et un test peut
//! fixer le jour.

use crate::dictionary::UserDictionary;
use crate::modes::{id_of, new_id};
use crate::retention;
use crate::settings::AppSettings;
use crate::text::word_count;
use crate::translation::TranslationLanguage;
use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, SecondsFormat, TimeZone, Utc};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

/// Ce qu'il est advenu de la mise au propre d'une dictée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cleanup {
    Model,
    Skipped,
    Short,
    None,
    Translated,
    Fallback,
    Rejected,
    Failed,
}

/// Temps d'une dictée après le relâchement de la touche, et ce qu'il est advenu de la mise au propre.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictationTiming {
    /// Secondes.
    pub transcription: f64,
    /// Secondes passées dans le modèle (0 sans appel).
    pub cleanup: f64,
    /// Du relâchement au collage.
    pub total: f64,
    pub outcome: Cleanup,
}

fn iso<S: Serializer>(date: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&date.to_rfc3339_opts(SecondsFormat::Secs, true))
}

fn parse_date(v: Option<&Value>) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(v?.as_str()?).ok().map(|d| d.with_timezone(&Utc))
}

/// Une dictée terminée, telle qu'elle est gardée dans l'historique (sur cet ordinateur uniquement).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Dictation {
    pub id: String,
    #[serde(serialize_with = "iso")]
    pub date: DateTime<Utc>,
    #[serde(rename = "appName")]
    pub app_name: String,
    #[serde(rename = "bundleID", skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    #[serde(rename = "rawText")]
    pub raw_text: String,
    #[serde(rename = "finalText")]
    pub final_text: String,
    pub words: i64,
    /// Secondes de parole.
    pub duration: f64,
    /// Corrections automatiques appliquées.
    pub corrections: i64,
    /// Langue de collage, si la dictée a été traduite.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation: Option<TranslationLanguage>,
    /// Temps après le relâchement (absent dans les anciennes dictées).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<DictationTiming>,
}

impl Dictation {
    pub fn new(date: DateTime<Utc>, app_name: &str, raw_text: &str, final_text: &str, words: i64, duration: f64, corrections: i64) -> Self {
        Self {
            id: new_id(),
            date,
            app_name: app_name.into(),
            bundle_id: None,
            raw_text: raw_text.into(),
            final_text: final_text.into(),
            words,
            duration,
            corrections,
            translation: None,
            timing: None,
        }
    }

    /// Faux quand seule la fiche chiffrée est gardée (réglage « Garder le texte des dictées » décoché).
    pub fn has_text(&self) -> bool {
        !self.final_text.is_empty()
    }

    pub fn words_per_minute(&self) -> i64 {
        if self.duration > 0.0 {
            (self.words as f64 / (self.duration / 60.0)).round() as i64
        } else {
            0
        }
    }

    /// Lecture tolérante : un champ absent ou ajouté par une version future ne fait jamais perdre l'historique.
    /// `None` si la date manque ou si ce n'est pas un objet.
    pub fn from_value(v: &Value) -> Option<Self> {
        let o = v.as_object()?;
        let date = parse_date(o.get("date"))?;
        let s = |k: &str| o.get(k).and_then(Value::as_str).map(str::to_string);
        let final_text = s("finalText").unwrap_or_default();
        Some(Self {
            id: id_of(o.get("id")),
            date,
            app_name: s("appName").unwrap_or_else(|| "Inconnue".into()),
            bundle_id: s("bundleID"),
            raw_text: s("rawText").unwrap_or_else(|| final_text.clone()),
            words: o.get("words").and_then(Value::as_i64).unwrap_or_else(|| word_count(&final_text) as i64),
            final_text,
            duration: o.get("duration").and_then(Value::as_f64).unwrap_or(0.0),
            corrections: o.get("corrections").and_then(Value::as_i64).unwrap_or(0),
            translation: o.get("translation").and_then(|v| serde_json::from_value(v.clone()).ok()),
            timing: o.get("timing").and_then(|v| serde_json::from_value(v.clone()).ok()),
        })
    }
}

/// Ce qui reste d'un jour dont les dictées ont été supprimées : des totaux, jamais un texte. Le tableau de bord les ajoute
/// à l'historique pour que ses chiffres (mots dictés, temps gagné, vitesse, série de jours) restent justes après une purge.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DayTotal {
    /// Début du jour.
    #[serde(serialize_with = "iso")]
    pub day: DateTime<Utc>,
    pub dictations: i64,
    pub words: i64,
    /// Secondes de parole.
    pub duration: f64,
    pub corrections: i64,
    /// Mots par appli.
    pub apps: BTreeMap<String, i64>,
}

impl DayTotal {
    /// Lecture tolérante : un champ absent vaut zéro, pour ne jamais écarter un jour entier (et faire baisser les chiffres).
    pub fn from_value(v: &Value) -> Option<Self> {
        let o = v.as_object()?;
        Some(Self {
            day: parse_date(o.get("day"))?,
            dictations: o.get("dictations").and_then(Value::as_i64).unwrap_or(0),
            words: o.get("words").and_then(Value::as_i64).unwrap_or(0),
            duration: o.get("duration").and_then(Value::as_f64).unwrap_or(0.0),
            corrections: o.get("corrections").and_then(Value::as_i64).unwrap_or(0),
            apps: o.get("apps").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default(),
        })
    }
}

/// Lit une liste en ignorant les éléments illisibles (au lieu de faire échouer toute la liste).
pub fn lossy_list<T>(json: &str, item: impl Fn(&Value) -> Option<T>) -> Result<Vec<T>, serde_json::Error> {
    let value: Value = serde_json::from_str(json)?;
    let array = value.as_array().ok_or_else(|| serde::de::Error::custom("une liste est attendue"))?;
    Ok(array.iter().filter_map(item).collect())
}

/// Le calendrier : un fuseau horaire. Les jours sont ceux du fuseau, comme `Calendar.current` dans Scribe.
pub struct Calendar<Tz: TimeZone> {
    pub tz: Tz,
}

impl<Tz: TimeZone> Calendar<Tz> {
    pub fn new(tz: Tz) -> Self {
        Self { tz }
    }

    fn local(&self, instant: DateTime<Utc>) -> NaiveDateTime {
        instant.with_timezone(&self.tz).naive_local()
    }

    fn to_instant(&self, local: NaiveDateTime) -> DateTime<Utc> {
        // Une heure locale qui n'existe pas (changement d'heure) ou existe deux fois : la première heure valide.
        self.tz
            .from_local_datetime(&local)
            .earliest()
            .or_else(|| self.tz.from_local_datetime(&local).latest())
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|| local.and_utc())
    }

    pub fn day_of(&self, instant: DateTime<Utc>) -> NaiveDate {
        self.local(instant).date()
    }

    /// Début du jour (heure locale) qui contient cet instant.
    pub fn start_of_day(&self, instant: DateTime<Utc>) -> DateTime<Utc> {
        self.to_instant(self.day_of(instant).and_hms_opt(0, 0, 0).unwrap_or_default())
    }

    /// Même heure locale, `days` jours plus tard (ou plus tôt).
    pub fn add_days(&self, instant: DateTime<Utc>, days: i64) -> DateTime<Utc> {
        self.to_instant(self.local(instant) + Duration::days(days))
    }

    pub fn same_day(&self, a: DateTime<Utc>, b: DateTime<Utc>) -> bool {
        self.day_of(a) == self.day_of(b)
    }
}

/// Conserver seulement les N derniers jours de l'historique (aujourd'hui compris) : le calcul, sans lecture ni écriture.
pub mod retention_calc {
    use super::*;

    /// Par défaut : un mois.
    pub const DEFAULT_DAYS: i64 = 30;

    /// Début du premier jour gardé : ce qui est plus ancien est supprimé.
    pub fn cutoff<Tz: TimeZone>(cal: &Calendar<Tz>, keep_days: i64, now: DateTime<Utc>) -> DateTime<Utc> {
        cal.add_days(cal.start_of_day(now), -(retention::clamped(keep_days) - 1))
    }

    /// Les totaux de ces dictées, jour par jour (aucun texte), du plus récent au plus ancien.
    pub fn totals<Tz: TimeZone>(cal: &Calendar<Tz>, items: &[Dictation]) -> Vec<DayTotal> {
        let mut by_day: HashMap<DateTime<Utc>, DayTotal> = HashMap::new();
        for item in items {
            let day = cal.start_of_day(item.date);
            let total = by_day.entry(day).or_insert_with(|| DayTotal {
                day,
                dictations: 0,
                words: 0,
                duration: 0.0,
                corrections: 0,
                apps: BTreeMap::new(),
            });
            total.dictations += 1;
            total.words += item.words;
            total.duration += item.duration;
            total.corrections += item.corrections;
            *total.apps.entry(item.app_name.clone()).or_insert(0) += item.words;
        }
        sorted_desc(by_day.into_values().collect())
    }

    /// Ajoute des totaux à ceux déjà gardés ; deux totaux du même jour s'additionnent.
    pub fn merged(existing: &[DayTotal], added: &[DayTotal]) -> Vec<DayTotal> {
        let mut by_day: HashMap<DateTime<Utc>, DayTotal> = HashMap::new();
        for e in existing {
            by_day.entry(e.day).or_insert_with(|| e.clone());
        }
        for add in added {
            match by_day.get_mut(&add.day) {
                None => {
                    by_day.insert(add.day, add.clone());
                }
                Some(total) => {
                    total.dictations += add.dictations;
                    total.words += add.words;
                    total.duration += add.duration;
                    total.corrections += add.corrections;
                    for (app, words) in &add.apps {
                        *total.apps.entry(app.clone()).or_insert(0) += words;
                    }
                }
            }
        }
        sorted_desc(by_day.into_values().collect())
    }

    fn sorted_desc(mut days: Vec<DayTotal>) -> Vec<DayTotal> {
        days.sort_by_key(|d| std::cmp::Reverse(d.day));
        days
    }
}

use retention_calc as hr;

/// Part d'une appli dans les mots dictés.
#[derive(Debug, Clone, PartialEq)]
pub struct AppShare {
    pub name: String,
    pub words: i64,
    pub share: f64,
}

/// Tout ce que l'appli garde, en mémoire (le stockage sur disque est ailleurs).
#[derive(Debug, Clone, Default)]
pub struct Store {
    pub history: Vec<Dictation>,
    /// Totaux par jour des dictées purgées de l'historique (aucun texte) : ils gardent justes les chiffres du tableau de bord.
    pub archive: Vec<DayTotal>,
    pub dictionary: UserDictionary,
    pub settings: AppSettings,
    /// Jour de la dernière application de la durée de conservation (une fois par jour suffit).
    pub retention_day: Option<NaiveDate>,
    /// Dernier texte collé, gardé en mémoire seulement : « Copier la dernière dictée » marche même quand le texte n'est pas gardé.
    pub last_dictation_text: Option<String>,
}

/// Ce qu'un changement des réglages de l'historique supprimerait ou effacerait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryChange {
    /// Dictées supprimées (plus anciennes que la durée).
    pub purged: usize,
    /// Dictées gardées dont le texte serait effacé.
    pub erased: usize,
}

impl Store {
    // MARK: Historique

    /// Sans « Garder le texte des dictées », seule la fiche chiffrée (date, appli, mots, durée, temps) est gardée.
    pub fn add<Tz: TimeZone>(&mut self, cal: &Calendar<Tz>, now: DateTime<Utc>, dictation: Dictation) {
        self.last_dictation_text = Some(dictation.final_text.clone());
        let mut kept = dictation;
        if !self.settings.keep_dictation_text {
            kept.raw_text.clear();
            kept.final_text.clear();
        }
        self.history.insert(0, kept);
        self.apply_retention_if_due(cal, now);
    }

    pub fn delete(&mut self, id: &str) {
        // La dernière dictée supprimée ne doit pas rester copiable depuis la barre des menus.
        if self.history.first().is_some_and(|d| d.id == id) {
            self.last_dictation_text = None;
        }
        self.history.retain(|d| d.id != id);
    }

    /// Tout effacer : les dictées et les totaux par jour (les chiffres du tableau de bord repartent à zéro).
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.archive.clear();
        self.last_dictation_text = None;
    }

    /// Les dictées que l'on supprimerait en ne gardant que `keep_days` jours : pour l'aperçu avant de confirmer.
    pub fn dictations_to_purge<Tz: TimeZone>(&self, cal: &Calendar<Tz>, now: DateTime<Utc>, keep_days: i64) -> Vec<&Dictation> {
        let cut = hr::cutoff(cal, keep_days, now);
        self.history.iter().filter(|d| d.date < cut).collect()
    }

    /// Supprime les dictées plus anciennes que `keep_days` jours, texte compris, et garde leurs totaux par jour. Renvoie leur nombre.
    pub fn purge_history<Tz: TimeZone>(&mut self, cal: &Calendar<Tz>, now: DateTime<Utc>, keep_days: i64) -> usize {
        let doomed: Vec<Dictation> = self.dictations_to_purge(cal, now, keep_days).into_iter().cloned().collect();
        if doomed.is_empty() {
            return 0;
        }
        let ids: HashSet<&str> = doomed.iter().map(|d| d.id.as_str()).collect();
        self.history.retain(|d| !ids.contains(d.id.as_str()));
        self.archive = hr::merged(&self.archive, &hr::totals(cal, &doomed));
        doomed.len()
    }

    /// Ce qu'un changement des réglages de l'historique supprimerait ou effacerait : pour l'aperçu avant de confirmer.
    pub fn preview_history_change<Tz: TimeZone>(
        &self,
        cal: &Calendar<Tz>,
        now: DateTime<Utc>,
        keep_text: bool,
        keep_days: Option<i64>,
    ) -> HistoryChange {
        let cutoff = keep_days.map(|d| hr::cutoff(cal, d, now));
        let purged = cutoff.map_or(0, |cut| self.history.iter().filter(|d| d.date < cut).count());
        let erased =
            if keep_text { 0 } else { self.history.iter().filter(|d| d.has_text() && cutoff.is_none_or(|cut| d.date >= cut)).count() };
        HistoryChange { purged, erased }
    }

    /// Enregistre les réglages de l'historique et les applique : dictées anciennes supprimées (totaux gardés), textes effacés
    /// (la fiche chiffrée reste). Définitif : l'appelant a montré l'aperçu avant.
    pub fn apply_history_change<Tz: TimeZone>(&mut self, cal: &Calendar<Tz>, now: DateTime<Utc>, keep_text: bool, keep_days: Option<i64>) {
        self.settings.keep_dictation_text = keep_text;
        self.settings.history_keep_days = keep_days.map(retention::clamped);
        self.retention_day = None;
        self.apply_retention_if_due(cal, now);
        if !keep_text {
            for d in &mut self.history {
                d.raw_text.clear();
                d.final_text.clear();
            }
            self.last_dictation_text = None;
        }
    }

    /// Applique la durée de conservation choisie, une fois par jour (au lancement, à chaque dictée, à l'ouverture des pages).
    pub fn apply_retention_if_due<Tz: TimeZone>(&mut self, cal: &Calendar<Tz>, now: DateTime<Utc>) {
        let Some(days) = self.settings.history_keep_days else { return };
        let today = cal.day_of(now);
        if self.retention_day == Some(today) {
            return;
        }
        self.retention_day = Some(today);
        self.purge_history(cal, now, days);
    }

    // MARK: Chiffres du tableau de bord

    /// Dictées gardées avec leur fiche, plus celles dont il ne reste que les totaux du jour.
    pub fn dictation_count(&self) -> i64 {
        self.history.len() as i64 + self.archive.iter().map(|d| d.dictations).sum::<i64>()
    }

    pub fn total_words(&self) -> i64 {
        self.history.iter().map(|d| d.words).sum::<i64>() + self.archive.iter().map(|d| d.words).sum::<i64>()
    }

    pub fn words_on<Tz: TimeZone>(&self, cal: &Calendar<Tz>, day: DateTime<Utc>) -> i64 {
        self.history.iter().filter(|d| cal.same_day(d.date, day)).map(|d| d.words).sum::<i64>()
            + self.archive.iter().filter(|d| cal.same_day(d.day, day)).map(|d| d.words).sum::<i64>()
    }

    pub fn today_words<Tz: TimeZone>(&self, cal: &Calendar<Tz>, now: DateTime<Utc>) -> i64 {
        self.words_on(cal, now)
    }

    pub fn yesterday_words<Tz: TimeZone>(&self, cal: &Calendar<Tz>, now: DateTime<Utc>) -> i64 {
        self.words_on(cal, cal.add_days(now, -1))
    }

    /// Mots d'hier jusqu'à la même heure qu'en ce moment (pour comparer à armes égales).
    pub fn yesterday_words_so_far<Tz: TimeZone>(&self, cal: &Calendar<Tz>, now: DateTime<Utc>) -> i64 {
        let same_time_yesterday = cal.add_days(now, -1);
        let start = cal.start_of_day(same_time_yesterday);
        self.history.iter().filter(|d| d.date >= start && d.date <= same_time_yesterday).map(|d| d.words).sum()
    }

    /// Écart en % avec hier à la même heure ; `None` quand hier, à cette heure-là, était à zéro.
    pub fn today_change<Tz: TimeZone>(&self, cal: &Calendar<Tz>, now: DateTime<Utc>) -> Option<i64> {
        let reference = self.yesterday_words_so_far(cal, now);
        if reference <= 0 {
            return None;
        }
        Some(((self.today_words(cal, now) - reference) as f64 / reference as f64 * 100.0).round() as i64)
    }

    pub fn average_wpm(&self) -> i64 {
        let seconds = self.history.iter().map(|d| d.duration).sum::<f64>() + self.archive.iter().map(|d| d.duration).sum::<f64>();
        if seconds > 0.0 {
            (self.total_words() as f64 / (seconds / 60.0)).round() as i64
        } else {
            0
        }
    }

    /// Temps qu'il aurait fallu pour taper ces mots au clavier, moins le temps passé à parler (en secondes).
    pub fn time_saved(&self) -> f64 {
        let typing = self.settings.typing_wpm.max(1) as f64;
        // Une dictée purgée n'a plus que les totaux de son jour : même calcul sur le jour entier.
        let saved = |words: i64, duration: f64| (words as f64 / typing * 60.0 - duration).max(0.0);
        self.history.iter().map(|d| saved(d.words, d.duration)).sum::<f64>()
            + self.archive.iter().map(|d| saved(d.words, d.duration)).sum::<f64>()
    }

    /// Les 14 derniers jours (le plus ancien d'abord) avec leurs mots.
    pub fn last_days<Tz: TimeZone>(&self, cal: &Calendar<Tz>, now: DateTime<Utc>) -> Vec<(DateTime<Utc>, i64)> {
        let today = cal.start_of_day(now);
        (0..14).rev().map(|offset| cal.add_days(today, -offset)).map(|day| (day, self.words_on(cal, day))).collect()
    }

    /// Jours d'affilée avec au moins une dictée, jusqu'à aujourd'hui (ou hier si rien encore aujourd'hui).
    pub fn streak<Tz: TimeZone>(&self, cal: &Calendar<Tz>, now: DateTime<Utc>) -> i64 {
        let days: HashSet<NaiveDate> = self
            .history
            .iter()
            .map(|d| cal.day_of(d.date))
            .chain(self.archive.iter().filter(|d| d.dictations > 0).map(|d| cal.day_of(d.day)))
            .collect();
        let mut day = cal.day_of(now);
        if !days.contains(&day) {
            day = day.pred_opt().unwrap_or(day);
        }
        let mut count = 0;
        while days.contains(&day) {
            count += 1;
            day = day.pred_opt().unwrap_or(day);
            if count > 100_000 {
                break;
            }
        }
        count
    }

    /// Les cinq applis qui reçoivent le plus de mots (à égalité : l'ordre alphabétique).
    pub fn top_apps(&self) -> Vec<AppShare> {
        let total = self.total_words().max(1);
        let mut grouped: BTreeMap<String, i64> = BTreeMap::new();
        for d in &self.history {
            *grouped.entry(d.app_name.clone()).or_insert(0) += d.words;
        }
        for day in &self.archive {
            for (app, words) in &day.apps {
                *grouped.entry(app.clone()).or_insert(0) += words;
            }
        }
        let mut list: Vec<(String, i64)> = grouped.into_iter().collect();
        list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        list.into_iter().take(5).map(|(name, words)| AppShare { name, words, share: words as f64 / total as f64 }).collect()
    }

    pub fn corrections_this_week<Tz: TimeZone>(&self, cal: &Calendar<Tz>, now: DateTime<Utc>) -> i64 {
        let start = cal.add_days(cal.start_of_day(now), -6);
        self.history.iter().filter(|d| d.date >= start).map(|d| d.corrections).sum::<i64>()
            + self.archive.iter().filter(|d| d.day >= start).map(|d| d.corrections).sum::<i64>()
    }

    /// Les noms des applis de l'historique, sans doublon, par ordre alphabétique.
    pub fn app_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.history.iter().map(|d| d.app_name.clone()).collect::<HashSet<_>>().into_iter().collect();
        names.sort();
        names
    }
}
