//! Les passages du banc de dictée gardés sur le disque, dans `bancs/` du dossier de données (un fichier par passage, comme Scribe).
//! Le fichier ne contient que les phrases du banc : lisible, sans texte dicté.

use scriibe_core::bench_archive::BenchRun;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Default)]
struct State {
    /// Du plus récent au plus ancien.
    runs: Vec<BenchRun>,
    /// Supprimés dans cette session : le banc qui les complète encore ne les recrée pas.
    deleted: HashSet<String>,
    /// Les fichiers lus pour chaque passage (copie faite à la main comprise), tous effacés avec lui.
    files: HashMap<String, Vec<PathBuf>>,
}

pub struct BenchStore {
    folder: PathBuf,
    state: Mutex<State>,
}

fn stamp(run: &BenchRun) -> String {
    run.date.format("%Y-%m-%dT%H-%M-%SZ").to_string()
}

impl BenchStore {
    pub fn standard_folder(data: &Path) -> PathBuf {
        data.join("bancs")
    }

    /// Lit les passages du dossier. Un fichier illisible est laissé tel quel et ignoré ; un passage copié à la main n'est montré qu'une
    /// fois ; un passage sans phrase lisible ne l'est pas.
    pub fn open(folder: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&folder);
        let mut state = State::default();
        let mut seen = HashSet::new();
        let mut files: Vec<PathBuf> = std::fs::read_dir(&folder)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .collect();
        files.sort();
        for file in files {
            let Some(run) = std::fs::read_to_string(&file)
                .ok()
                .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                .and_then(|v| BenchRun::from_value(&v))
            else {
                continue;
            };
            state.files.entry(run.id.clone()).or_default().push(file);
            if !run.entries.is_empty() && seen.insert(run.id.clone()) {
                state.runs.push(run);
            }
        }
        state.runs.sort_by_key(|r| std::cmp::Reverse(r.date));
        Self { folder, state: Mutex::new(state) }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Les passages, du plus récent au plus ancien.
    pub fn runs(&self) -> Vec<BenchRun> {
        self.state().runs.clone()
    }

    pub fn get(&self, id: &str) -> Option<BenchRun> {
        self.state().runs.iter().find(|r| r.id == id).cloned()
    }

    fn path(&self, run: &BenchRun) -> PathBuf {
        self.folder.join(format!("banc-{}-{}.json", stamp(run), run.id.chars().take(8).collect::<String>()))
    }

    /// Ajoute ou remplace le passage (un passage se complète phrase après phrase). Sans effet s'il a été supprimé.
    pub fn save(&self, run: BenchRun) {
        let mut state = self.state();
        if state.deleted.contains(&run.id) {
            return;
        }
        match state.runs.iter().position(|r| r.id == run.id) {
            Some(i) => state.runs[i] = run.clone(),
            None => state.runs.push(run.clone()),
        }
        state.runs.sort_by_key(|r| std::cmp::Reverse(r.date));
        if let Ok(json) = serde_json::to_string_pretty(&run) {
            let path = self.path(&run);
            let _ = std::fs::write(&path, json);
            let files = state.files.entry(run.id.clone()).or_default();
            if !files.contains(&path) {
                files.push(path);
            }
        }
    }

    /// Suppression demandée : le passage ne revient pas, même si le banc le complète encore.
    pub fn delete(&self, id: &str) {
        self.state().deleted.insert(id.to_string());
        self.discard(id);
    }

    /// Retire un passage devenu vide (« Relire » jusqu'à la première phrase) ; il revient si une phrase est relue.
    pub fn discard(&self, id: &str) {
        let mut state = self.state();
        for file in state.files.remove(id).unwrap_or_default() {
            let _ = std::fs::remove_file(file);
        }
        state.runs.retain(|r| r.id != id);
    }

    pub fn is_deleted(&self, id: &str) -> bool {
        self.state().deleted.contains(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use scriibe_core::bench_archive::{BenchSetup, RunEntry};
    use scriibe_core::history::{Cleanup, DictationTiming};

    fn run(minutes: i64, entries: usize) -> BenchRun {
        let entry = |i: usize| RunEntry {
            category: "Propre".into(),
            text: format!("phrase {i}"),
            raw: "brut".into(),
            pasted: "collé".into(),
            timing: DictationTiming { transcription: 1.0, cleanup: 0.5, total: 1.6, outcome: Cleanup::Model },
        };
        BenchRun::new(Utc.timestamp_opt(1_800_000_000 + minutes * 60, 0).unwrap(), BenchSetup::default(), (0..entries).map(entry).collect())
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("scriibe-bancs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_run_is_kept_on_disk_completed_and_listed_newest_first() {
        let dir = temp("keep");
        let store = BenchStore::open(dir.clone());
        let (old, new) = (run(0, 2), run(10, 3));
        store.save(old.clone());
        store.save(new.clone());
        let mut longer = old.clone();
        longer.entries.extend(run(0, 1).entries);
        store.save(longer.clone());
        let reread = BenchStore::open(dir.clone());
        assert_eq!(reread.runs().iter().map(|r| r.id.clone()).collect::<Vec<_>>(), vec![new.id.clone(), old.id.clone()]);
        assert_eq!(reread.get(&old.id).unwrap().entries.len(), 3, "un passage se complète : le fichier est remplacé");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_copied_file_is_shown_once_and_deleting_removes_every_copy_and_blocks_late_saves() {
        let dir = temp("copy");
        let store = BenchStore::open(dir.clone());
        let r = run(0, 2);
        store.save(r.clone());
        let file = std::fs::read_dir(&dir).unwrap().next().unwrap().unwrap().path();
        std::fs::copy(&file, dir.join("copie.json")).unwrap();
        let reread = BenchStore::open(dir.clone());
        assert_eq!(reread.runs().len(), 1, "un passage copié à la main n'est montré qu'une fois");
        reread.delete(&r.id);
        assert!(BenchStore::open(dir.clone()).runs().is_empty(), "supprimé sur le disque, copies comprises");
        reread.save(r.clone());
        assert!(reread.runs().is_empty(), "un passage supprimé n'est pas recréé par ce qui arrive après");
        assert!(reread.is_deleted(&r.id));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_unreadable_file_is_ignored_and_left_alone_and_an_empty_run_is_not_listed() {
        let dir = temp("junk");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("abime.json"), "pas du json").unwrap();
        let store = BenchStore::open(dir.clone());
        store.save(run(0, 0));
        let reread = BenchStore::open(dir.clone());
        assert!(reread.runs().is_empty());
        assert!(dir.join("abime.json").exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn discarding_a_run_that_became_empty_removes_it_until_a_sentence_is_read_again() {
        let dir = temp("discard");
        let store = BenchStore::open(dir.clone());
        let r = run(0, 1);
        store.save(r.clone());
        store.discard(&r.id);
        assert!(BenchStore::open(dir.clone()).runs().is_empty());
        store.save(r.clone());
        assert_eq!(BenchStore::open(dir.clone()).runs().len(), 1, "il revient si une phrase est relue");
        let _ = std::fs::remove_dir_all(dir);
    }
}
