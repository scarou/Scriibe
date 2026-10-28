//! Les passages du banc de dictée gardés d'une version à l'autre, et leur comparaison phrase par phrase. Port de `BenchArchive.swift`
//! (`BenchSetup`, `BenchRun`, `BenchComparison`, `WordDiff`), sans lecture ni écriture de fichiers (voir `scriibe-platform`).
//!
//! Chaque passage est gardé avec ses réglages ; chaque phrase garde son texte : la comparaison reconnaît une phrase par ce qu'on a lu,
//! même si la liste du banc change d'une version à l'autre. Le format des fichiers est celui de Scribe.

use crate::bench::{self, Check};
use crate::history::DictationTiming;
use crate::modes::new_id;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::ser::Serializer;
use serde::Serialize;
use serde_json::Value;

/// Les réglages d'un passage, écrits comme on les lit (pour reconnaître une version, un modèle, un mode).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct BenchSetup {
    pub version: String,
    pub transcription: String,
    pub cleanup: String,
    pub mode: String,
    /// Empreinte de la consigne et des exemples du mode : même nom, consigne différente → réglages différents.
    #[serde(rename = "modeDetail")]
    pub mode_detail: String,
    #[serde(rename = "skipWhenClean")]
    pub skip_when_clean: bool,
    pub microphone: String,
}

impl BenchSetup {
    /// Les réglages qui changent le texte ou les temps (le micro aussi : il change ce que Whisper entend).
    pub fn summary(&self) -> String {
        let skip = if self.skip_when_clean { "sauter si propre" } else { "" };
        [self.version.as_str(), &self.transcription, &self.cleanup, &self.mode, skip, &self.microphone]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · ")
    }

    /// Lecture tolérante : un champ absent ou illisible reste vide.
    pub fn from_value(v: &Value) -> Self {
        let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        Self {
            version: s("version"),
            transcription: s("transcription"),
            cleanup: s("cleanup"),
            mode: s("mode"),
            mode_detail: s("modeDetail"),
            skip_when_clean: v.get("skipWhenClean").and_then(Value::as_bool).unwrap_or(false),
            microphone: s("microphone"),
        }
    }
}

/// Une phrase lue pendant un passage.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunEntry {
    pub category: String,
    pub text: String,
    pub raw: String,
    pub pasted: String,
    pub timing: DictationTiming,
    /// L'option « Sauter si déjà propre », activée avec ces réglages, aurait-elle sauté le modèle ?
    #[serde(rename = "wouldSkip")]
    pub would_skip: bool,
    /// Ce qui aurait été collé sans modèle (dictionnaire, signes dictés, typographie).
    #[serde(rename = "withoutModel")]
    pub without_model: String,
    /// Ce que le modèle a écrit quand la phrase a été sautée (lancé à côté, sans rien coller).
    #[serde(rename = "modelText", skip_serializing_if = "Option::is_none")]
    pub model_text: Option<String>,
    #[serde(rename = "modelSeconds", skip_serializing_if = "Option::is_none")]
    pub model_seconds: Option<f64>,
    #[serde(rename = "modelError", skip_serializing_if = "Option::is_none")]
    pub model_error: Option<String>,
}

impl RunEntry {
    /// Les deux versions d'une phrase jugée propre : (avec modèle, sans modèle) ; `None` si elle ne l'est pas, ou tant que la
    /// comparaison n'est pas revenue. Port de `ReadingBench.Result.versions`.
    pub fn versions(&self) -> Option<(&str, &str)> {
        if !self.would_skip {
            return None;
        }
        match self.timing.outcome {
            crate::history::Cleanup::Skipped => self.model_text.as_deref().map(|m| (m, self.pasted.as_str())),
            crate::history::Cleanup::Model => Some((self.pasted.as_str(), self.without_model.as_str())),
            _ => None,
        }
    }

    /// Lecture tolérante : sans texte lu ni texte collé, la phrase n'a pas de sens et elle est écartée.
    pub fn from_value(v: &Value) -> Option<Self> {
        let s = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
        Some(Self {
            text: s("text")?,
            pasted: s("pasted")?,
            category: s("category").unwrap_or_default(),
            raw: s("raw").unwrap_or_default(),
            would_skip: v.get("wouldSkip").and_then(Value::as_bool).unwrap_or(false),
            without_model: s("withoutModel").unwrap_or_default(),
            model_text: s("modelText"),
            model_seconds: v.get("modelSeconds").and_then(Value::as_f64),
            model_error: s("modelError"),
            timing: v.get("timing").and_then(|t| serde_json::from_value(t.clone()).ok()).unwrap_or(DictationTiming {
                transcription: 0.0,
                cleanup: 0.0,
                total: 0.0,
                outcome: crate::history::Cleanup::None,
            }),
        })
    }
}

fn iso<S: Serializer>(date: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&date.to_rfc3339_opts(SecondsFormat::Secs, true))
}

/// Un passage du banc tel qu'il est gardé sur le disque.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BenchRun {
    pub id: String,
    #[serde(serialize_with = "iso")]
    pub date: DateTime<Utc>,
    pub setup: BenchSetup,
    pub entries: Vec<RunEntry>,
    /// Les réglages ont changé pendant le passage : `setup` est celui de la première phrase.
    #[serde(rename = "settingsChanged")]
    pub settings_changed: bool,
}

/// Un contrôle appliqué à un texte collé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckResult {
    pub label: String,
    pub passed: bool,
}

impl BenchRun {
    pub fn new(date: DateTime<Utc>, setup: BenchSetup, entries: Vec<RunEntry>) -> Self {
        Self { id: new_id(), date, setup, entries, settings_changed: false }
    }

    /// Lecture tolérante : une phrase illisible est écartée, pas le passage entier. `None` sans identifiant ni date valides.
    pub fn from_value(v: &Value) -> Option<Self> {
        let id = crate::modes::parse_id(v.get("id"))?;
        let date = DateTime::parse_from_rfc3339(v.get("date")?.as_str()?).ok()?.with_timezone(&Utc);
        Some(Self {
            id,
            date,
            setup: v.get("setup").map(BenchSetup::from_value).unwrap_or_default(),
            entries: v
                .get("entries")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(RunEntry::from_value).collect())
                .unwrap_or_default(),
            settings_changed: v.get("settingsChanged").and_then(Value::as_bool).unwrap_or(false),
        })
    }

    /// « 19/10 14:32 · 1.12.0 (15) · LM Studio · gemma… · 17 phrase(s) » (heure en UTC ici : l'affichage local est fait par l'interface).
    pub fn label(&self) -> String {
        format!("{} · {} · {} phrase(s)", self.date.format("%d/%m %H:%M"), self.setup.summary(), self.entries.len())
    }

    /// Contrôles d'une phrase lue, d'après la liste actuelle du banc (aucun si la phrase n'y est plus).
    pub fn checks_for(entry: &RunEntry) -> Vec<CheckResult> {
        bench::phrases()
            .iter()
            .find(|p| p.text == entry.text)
            .map(|p| {
                p.checks.iter().map(|c: &Check| CheckResult { label: c.label().to_string(), passed: c.passes(&entry.pasted) }).collect()
            })
            .unwrap_or_default()
    }

    /// Contrôles passés et contrôles faits, sur tout le passage.
    pub fn check_score(&self) -> (usize, usize) {
        let all: Vec<CheckResult> = self.entries.iter().flat_map(Self::checks_for).collect();
        (all.iter().filter(|c| c.passed).count(), all.len())
    }

    /// Temps total moyen des phrases dont la mise au propre a eu ce résultat.
    pub fn average_total(&self, outcome: crate::history::Cleanup) -> Option<f64> {
        let totals: Vec<f64> = self.entries.iter().filter(|e| e.timing.outcome == outcome).map(|e| e.timing.total).collect();
        (!totals.is_empty()).then(|| totals.iter().sum::<f64>() / totals.len() as f64)
    }
}

/// Les phrases jugées propres où le modèle écrit autre chose que le texte sans modèle (ponctuation et casse comprises, espaces
/// insécables ignorés) : ce que l'option ferait perdre, ou gagner. Port de `ReadingBench.cleanButChanged`.
pub fn clean_but_changed(entries: &[RunEntry]) -> usize {
    entries.iter().filter_map(RunEntry::versions).filter(|(with, without)| normalized(with) != normalized(without)).count()
}

/// Le texte collé comparé sans les différences d'espaces (insécables, multiples) et d'apostrophes.
pub fn normalized(text: &str) -> String {
    text.replace('\u{00A0}', " ").replace('’', "'").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Le passage à comparer à `run` : le plus récent fait avec d'autres réglages, sinon le plus récent tout court ; les passages d'au
/// moins 3 phrases d'abord (un « Recommencer » après une phrase en laisse un tout petit). `runs` : du plus récent au plus ancien.
pub fn partner_of<'a>(runs: &'a [BenchRun], run: &BenchRun) -> Option<&'a BenchRun> {
    let others: Vec<&BenchRun> = runs.iter().filter(|r| r.id != run.id).collect();
    let full: Vec<&BenchRun> = others.iter().copied().filter(|r| r.entries.len() >= 3).collect();
    full.iter()
        .copied()
        .find(|r| r.setup != run.setup)
        .or_else(|| others.iter().copied().find(|r| r.setup != run.setup))
        .or_else(|| full.first().copied())
        .or_else(|| others.first().copied())
}

/// Le plus récent et son partenaire (le plus ancien d'abord dans le résultat).
pub fn default_pair(runs: &[BenchRun]) -> Option<(&BenchRun, &BenchRun)> {
    let newer = runs.first()?;
    Some((partner_of(runs, newer)?, newer))
}

/// Une phrase dans deux passages.
#[derive(Debug, Clone)]
pub struct ComparisonRow {
    pub category: String,
    pub text: String,
    pub older: Option<RunEntry>,
    pub newer: Option<RunEntry>,
    pub older_checks: Vec<CheckResult>,
    pub newer_checks: Vec<CheckResult>,
}

impl ComparisonRow {
    fn new(category: String, text: String, older: Option<RunEntry>, newer: Option<RunEntry>) -> Self {
        let older_checks = older.as_ref().map(BenchRun::checks_for).unwrap_or_default();
        let newer_checks = newer.as_ref().map(BenchRun::checks_for).unwrap_or_default();
        Self { category, text, older, newer, older_checks, newer_checks }
    }

    /// Lue dans les deux passages.
    pub fn in_both(&self) -> bool {
        self.older.is_some() && self.newer.is_some()
    }

    /// Le texte collé a changé (espaces insécables et apostrophes ignorés).
    pub fn changed(&self) -> bool {
        match (&self.older, &self.newer) {
            (Some(a), Some(b)) => normalized(&a.pasted) != normalized(&b.pasted),
            _ => true,
        }
    }

    fn flips(&self, from: bool) -> Vec<String> {
        if !self.in_both() {
            return Vec::new();
        }
        self.older_checks
            .iter()
            .zip(&self.newer_checks)
            .filter(|(a, b)| a.passed == from && b.passed != from)
            .map(|(a, _)| a.label.clone())
            .collect()
    }

    /// Contrôles réussis avant et ratés après.
    pub fn regressions(&self) -> Vec<String> {
        self.flips(true)
    }

    /// Contrôles ratés avant et réussis après.
    pub fn improvements(&self) -> Vec<String> {
        self.flips(false)
    }
}

/// Dans l'ordre du banc actuel, puis les phrases qui n'y sont plus.
pub fn compare(older: &BenchRun, newer: &BenchRun) -> Vec<ComparisonRow> {
    let mut texts: Vec<String> = bench::phrases().iter().map(|p| p.text.clone()).collect();
    for e in older.entries.iter().chain(&newer.entries) {
        if !texts.contains(&e.text) {
            texts.push(e.text.clone());
        }
    }
    texts
        .into_iter()
        .filter_map(|text| {
            let a = older.entries.iter().find(|e| e.text == text).cloned();
            let b = newer.entries.iter().find(|e| e.text == text).cloned();
            let category = b.as_ref().or(a.as_ref())?.category.clone();
            Some(ComparisonRow::new(category, text, a, b))
        })
        .collect()
}

/// Un mot d'un texte comparé, surligné s'il a été ajouté ou retiré.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiffWord {
    pub text: String,
    pub changed: bool,
}

/// Les mots (les retours à la ligne deviennent « ⏎ », comme dans les résultats du banc).
fn words(text: &str) -> Vec<String> {
    text.replace('\n', " ⏎ ").split_whitespace().map(str::to_string).collect()
}

/// Mots ajoutés ou retirés entre deux textes (plus longue suite commune, mot à mot) : ceux de l'ancien, puis ceux du nouveau.
pub fn word_diff(old: &str, new: &str) -> (Vec<DiffWord>, Vec<DiffWord>) {
    let (a, b) = (words(old), words(new));
    let key = |w: &String| w.replace('’', "'");
    let (ka, kb): (Vec<String>, Vec<String>) = (a.iter().map(key).collect(), b.iter().map(key).collect());
    // Table de la plus longue suite commune ; les dictées du banc font quelques dizaines de mots.
    let mut table = vec![vec![0usize; kb.len() + 1]; ka.len() + 1];
    for i in (0..ka.len()).rev() {
        for j in (0..kb.len()).rev() {
            table[i][j] = if ka[i] == kb[j] { table[i + 1][j + 1] + 1 } else { table[i + 1][j].max(table[i][j + 1]) };
        }
    }
    let (mut kept_a, mut kept_b) = (vec![false; ka.len()], vec![false; kb.len()]);
    let (mut i, mut j) = (0, 0);
    while i < ka.len() && j < kb.len() {
        if ka[i] == kb[j] {
            kept_a[i] = true;
            kept_b[j] = true;
            i += 1;
            j += 1;
        } else if table[i + 1][j] >= table[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    let mark =
        |words: Vec<String>, kept: Vec<bool>| words.into_iter().zip(kept).map(|(text, kept)| DiffWord { text, changed: !kept }).collect();
    (mark(a, kept_a), mark(b, kept_b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::Cleanup;
    use chrono::TimeZone;

    fn timing() -> DictationTiming {
        DictationTiming { transcription: 1.0, cleanup: 0.5, total: 1.6, outcome: Cleanup::Model }
    }

    fn entry(phrase: usize, pasted: &str) -> RunEntry {
        let p = &bench::phrases()[phrase];
        RunEntry {
            category: p.category.clone(),
            text: p.text.clone(),
            raw: String::new(),
            pasted: pasted.into(),
            timing: timing(),
            would_skip: false,
            without_model: String::new(),
            model_text: None,
            model_seconds: None,
            model_error: None,
        }
    }

    fn run(setup: &BenchSetup, minutes: i64, entries: &[(usize, &str)]) -> BenchRun {
        BenchRun {
            id: new_id(),
            date: Utc.timestamp_opt(1_800_000_000 + minutes * 60, 0).unwrap(),
            setup: setup.clone(),
            entries: entries.iter().map(|(i, t)| entry(*i, t)).collect(),
            settings_changed: false,
        }
    }

    fn setups() -> (BenchSetup, BenchSetup) {
        let a = BenchSetup { version: "1.11.0 (14)".into(), cleanup: "LM Studio · gemma".into(), ..BenchSetup::default() };
        let b = BenchSetup { version: "1.12.0 (15)".into(), ..a.clone() };
        (a, b)
    }

    // Les cas qui suivent sont ceux de `Checks.swift` (lignes 1030 à 1112) : mêmes passages, mêmes attentes.

    #[test]
    fn default_pair_is_the_newest_with_the_latest_run_made_with_other_settings() {
        let (a, b) = setups();
        let old = run(
            &a,
            0,
            &[
                (5, "Le rendez-vous est à 10 h, non pardon, à 11 h."),
                (7, "On part mardi selon la météo."),
                (9, "Je voulais te dire que le colis est bien arrivé."),
            ],
        );
        let same = run(&a, 5, &[(5, "Le rendez-vous est à 11 h.")]);
        let new = run(
            &b,
            10,
            &[(5, "Le rendez-vous est à 11 h."), (7, "On part lundi, enfin mardi."), (10, "Le colis (le gros) est arrivé.\nMerci encore")],
        );
        let mut runs = vec![new.clone(), same.clone(), old.clone()];
        runs.sort_by_key(|r| std::cmp::Reverse(r.date));
        let (older, newer) = default_pair(&runs).unwrap();
        assert_eq!((newer.id.as_str(), older.id.as_str()), (new.id.as_str(), old.id.as_str()), "le passage d'une phrase est écarté");
        // Le passage récent supprimé : le partenaire n'est jamais le passage lui-même.
        runs.retain(|r| r.id != new.id);
        assert_eq!(partner_of(&runs, &same).unwrap().id, old.id);
        assert_eq!(partner_of(&runs, &old).unwrap().id, same.id);
    }

    #[test]
    fn comparison_recognises_sentences_by_their_text_in_the_order_of_the_bench() {
        let (a, b) = setups();
        let old = run(
            &a,
            0,
            &[
                (5, "Le rendez-vous est à 10 h, non pardon, à 11 h."),
                (7, "On part mardi selon la météo."),
                (9, "Je voulais te dire que le colis est bien arrivé."),
            ],
        );
        let new = run(
            &b,
            10,
            &[(5, "Le rendez-vous est à 11 h."), (7, "On part lundi, enfin mardi."), (10, "Le colis (le gros) est arrivé.\nMerci encore")],
        );
        let rows = compare(&old, &new);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows.iter().filter(|r| r.in_both()).count(), 2);
        let by_text = |i: usize| rows.iter().find(|r| r.text == bench::phrases()[i].text).unwrap();
        assert!(by_text(9).newer.is_none());
        assert!(by_text(10).older.is_none());
        let (r5, r7) = (by_text(5), by_text(7));
        assert_eq!(r5.improvements(), vec!["dix heures retiré", "« non pardon » retiré"]);
        assert!(r5.regressions().is_empty());
        assert_eq!(r7.regressions(), vec!["lundi retiré"]);
        assert!(r7.changed());
        assert_eq!(old.check_score(), (5, 7), "contrôles du passage : 7 faits, 5 réussis");
    }

    #[test]
    fn words_added_or_removed_are_found_word_by_word() {
        let (old, new) = word_diff("Le rendez-vous est à dix heures.", "Le rendez-vous est à onze heures.");
        assert_eq!(old.iter().filter(|w| w.changed).map(|w| w.text.as_str()).collect::<Vec<_>>(), vec!["dix"]);
        assert_eq!(new.iter().filter(|w| w.changed).map(|w| w.text.as_str()).collect::<Vec<_>>(), vec!["onze"]);
        let (old, _) = word_diff("a\nb", "a b");
        assert_eq!(old.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(), vec!["a", "⏎", "b"], "retour à la ligne visible");
    }

    #[test]
    fn an_old_run_is_read_tolerantly() {
        let file = r#"{"id":"6F9619FF-8B86-D011-B42D-00CF4FC964FF","date":"2026-10-19T10:00:00Z","entries":[{"text":"a","pasted":"b","timing":"?"},{"text":"c"}]}"#;
        let run = BenchRun::from_value(&serde_json::from_str(file).unwrap()).unwrap();
        assert_eq!(run.entries.len(), 1, "la phrase illisible est écartée");
        assert_eq!(run.entries[0].timing.outcome, Cleanup::None);
        assert_eq!(run.setup, BenchSetup::default());
        assert_eq!(BenchSetup::from_value(&serde_json::json!({"version": 3})), BenchSetup::default(), "réglages illisibles → vides");
    }

    #[test]
    fn the_setup_is_described_as_it_is_read() {
        let setup = BenchSetup {
            version: "1.12.0 (15)".into(),
            transcription: "Whisper · ggml-large-v3-turbo".into(),
            cleanup: "LM Studio · gemma-4-e4b-it@q4_0".into(),
            mode: "Simple".into(),
            skip_when_clean: true,
            microphone: "Micro du Mac".into(),
            ..BenchSetup::default()
        };
        assert_eq!(
            setup.summary(),
            "1.12.0 (15) · Whisper · ggml-large-v3-turbo · LM Studio · gemma-4-e4b-it@q4_0 · Simple · sauter si propre · Micro du Mac"
        );
    }

    #[test]
    fn a_run_serialises_in_the_format_of_scribe() {
        let (a, _) = setups();
        let r = run(&a, 0, &[(0, "Je serai à la maison vers 19 h, on pourra dîner ensemble.")]);
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["date"], "2027-01-15T08:00:00Z");
        assert_eq!(json["entries"][0]["timing"]["outcome"], "model");
        assert_eq!(BenchRun::from_value(&json).unwrap(), r, "écrit puis relu à l'identique");
    }

    #[test]
    fn a_skipped_sentence_is_compared_with_what_the_model_wrote_and_a_modelled_one_with_the_text_without_model() {
        let mut skipped = entry(0, "Je serai là.");
        skipped.timing.outcome = Cleanup::Skipped;
        skipped.would_skip = true;
        assert_eq!(skipped.versions(), None, "la comparaison n'est pas revenue");
        skipped.model_text = Some("Je serai là !".into());
        assert_eq!(skipped.versions(), Some(("Je serai là !", "Je serai là.")));

        let mut modelled = entry(1, "Texte corrigé.");
        modelled.would_skip = true;
        modelled.without_model = "Texte corigé.".into();
        assert_eq!(modelled.versions(), Some(("Texte corrigé.", "Texte corigé.")));

        let mut to_redo = entry(2, "x");
        to_redo.without_model = "y".into();
        assert_eq!(to_redo.versions(), None, "jugée à reprendre : rien à comparer");

        assert_eq!(clean_but_changed(&[skipped, modelled, to_redo]), 2);
    }

    #[test]
    fn spaces_and_apostrophes_do_not_count_as_a_change() {
        let mut same = entry(0, "Tu viens\u{00A0}?");
        same.would_skip = true;
        same.without_model = "Tu  viens ?".into();
        assert_eq!(clean_but_changed(&[same]), 0);
    }

    #[test]
    fn the_comparison_with_the_model_is_written_and_read_in_the_format_of_scribe() {
        let (a, _) = setups();
        let mut r = run(&a, 0, &[(0, "Je serai là.")]);
        r.entries[0].timing.outcome = Cleanup::Skipped;
        r.entries[0].would_skip = true;
        r.entries[0].without_model = "Je serai là.".into();
        r.entries[0].model_text = Some("Je serai là !".into());
        r.entries[0].model_seconds = Some(1.25);
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["entries"][0]["wouldSkip"], true);
        assert_eq!(json["entries"][0]["modelText"], "Je serai là !");
        assert!(json["entries"][0].get("modelError").is_none(), "absent quand il n'y en a pas, comme chez Scribe");
        assert_eq!(BenchRun::from_value(&json).unwrap(), r);
    }
}
