//! Rejoue les cas de référence de `fixtures/` (tirés des vérifications de Scribe) contre le cœur Rust.
//!
//! Chaque cas a une fonction `fn` ; celles que le cœur ne sait pas encore faire sont listées dans `PENDING` : le test échoue
//! si un cas utilise une fonction qui n'est ni implémentée ni déclarée en attente, pour qu'aucun cas ne soit ignoré en silence.

use scriibe_core::dictionary::{self, Correction, UserDictionary};
use scriibe_core::spoken_signs::{self, ListBullet};
use scriibe_core::{text, typography};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

/// Fonctions pas encore portées, avec l'étape où elles arrivent.
const PENDING: &[&str] = &[
    "cleaner_accepts",
    "cleaner_strip",
    "accepts_translation",
    "skips_cleanup",
    "decode_settings",
    "settings_derived",
    "settings_roundtrip",
    "decode_history",
    "decode_history_entry",
    "decode_dictionary",
    "decode_day_totals",
    "history_roundtrip",
    "hotkey_sequence",
    "command_held",
    "media_decision",
    "media_new_players",
    "mic_level",
    "mic_resolve",
    "mic_label",
    "is_whisper_model",
    "distinct_models",
    "same_model",
    "llama_base_name",
    "llama_arguments",
    "llama_detect_models",
    "llama_assistant",
    "llama_status",
    "whisper_should_release",
    "cleanup_engine_display",
    "retention_cutoff",
    "retention_cutoff_equals",
    "history_scenario",
    "prompt_fingerprint",
    "overlay_origin_x",
    "folded_cards_toggle",
    "folded_cards_contains",
    "dictation_zone_clears",
    "invariant",
    "schtroumpf_apply",
    "schtroumpf_verb_form",
];

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn str_arg(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string)
}

/// Le texte d'entrée d'un cas : `text`, ou construit par `text_pattern`.
fn input_text(args: &Value) -> String {
    if let Some(t) = str_arg(args, "text") {
        return t;
    }
    let p = &args["text_pattern"];
    let s = |k: &str| p[k].as_str().unwrap_or("").to_string();
    let n = p["repeat"].as_u64().unwrap_or(0) as usize;
    // prefix + (separator × repeat, avec middle au milieu de la répétition) + suffix, comme Checks.swift.
    let sep = s("separator");
    let mut out = s("prefix");
    if s("middle").is_empty() {
        out.push_str(&sep.repeat(n));
    } else {
        out.push_str(&sep.repeat(n));
        out.push_str(&s("middle"));
        out.push_str(&sep.repeat(n));
    }
    out.push_str(&s("suffix"));
    out
}

fn dictionary_arg(args: &Value) -> UserDictionary {
    let d = &args["dictionary"];
    UserDictionary {
        words: d["words"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default(),
        corrections: d["corrections"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|c| Correction {
                        heard: c["heard"].as_str().unwrap_or("").into(),
                        written: c["written"].as_str().unwrap_or("").into(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn bullet_arg(args: &Value) -> ListBullet {
    match args.get("bullet").and_then(Value::as_str) {
        Some("dot") => ListBullet::Dot,
        Some("star") => ListBullet::Star,
        _ => ListBullet::Dash,
    }
}

/// Le résultat d'une fonction du chaînage : du texte, ou un booléen en fin de chaîne.
#[derive(Debug, PartialEq)]
enum Out {
    Text(String),
    Flag(bool),
}

fn run_step(name: &str, args: &Value, input: &str) -> Option<Out> {
    Some(match name {
        "french_typography" => Out::Text(typography::french_typography(input)),
        "known_hallucination" => Out::Flag(text::known_hallucination(input).is_some()),
        "spoken_signs_apply" => Out::Text(spoken_signs::apply(input, bullet_arg(args), true)),
        "apply_dictionary" => {
            let lists = args.get("lists").and_then(Value::as_bool).unwrap_or(true);
            Out::Text(dictionary::apply(&dictionary_arg(args), input, bullet_arg(args), lists).text)
        }
        "looks_clean" => Out::Flag(text::looks_clean(input)),
        _ => return None,
    })
}

fn run_case(case: &Value) -> Result<Option<()>, String> {
    let fns: Vec<String> = match &case["fn"] {
        Value::String(s) => vec![s.clone()],
        Value::Array(a) => a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        _ => return Err("fn illisible".into()),
    };
    if fns.iter().any(|f| PENDING.contains(&f.as_str())) {
        return Ok(None);
    }
    let args = &case["args"];
    let start = input_text(args);
    let compute = || -> Result<Out, String> {
        let mut current = Out::Text(start.clone());
        for (i, name) in fns.iter().enumerate() {
            let Out::Text(t) = &current else { return Err("chaînage après un booléen".into()) };
            let step_args = if i == 0 { args } else { &Value::Null };
            current = run_step(name, step_args, t).ok_or_else(|| format!("fonction inconnue : {name}"))?;
        }
        Ok(current)
    };
    let began = Instant::now();
    let got = compute()?;
    if let Some(prop) = case.get("property").and_then(Value::as_str) {
        return match prop {
            "idempotent" => {
                // Appliquer deux fois donne le même texte qu'une fois.
                let Out::Text(once) = &got else { return Err("idempotence sur un booléen".into()) };
                let again = run_step(&fns[0], args, once).ok_or("fonction inconnue")?;
                if again == got {
                    Ok(Some(()))
                } else {
                    Err(format!("pas idempotent : {got:?} puis {again:?}"))
                }
            }
            p if p.starts_with("max_duration_ms:") => {
                let limit: u128 = p["max_duration_ms:".len()..].parse().map_err(|_| "durée illisible")?;
                if began.elapsed().as_millis() < limit {
                    Ok(Some(()))
                } else {
                    Err(format!("trop lent : {} ms", began.elapsed().as_millis()))
                }
            }
            other => Err(format!("propriété inconnue : {other}")),
        };
    }
    let expected = &case["expected"];
    let ok = match (&got, expected) {
        (Out::Text(t), Value::String(e)) => match case.get("comparison").and_then(Value::as_str) {
            None | Some("equals") => t == e,
            Some("prefix") | Some("starts_with") => t.starts_with(e.as_str()),
            Some("ends_with") => t.ends_with(e.as_str()),
            Some("contains") => t.contains(e.as_str()),
            Some(other) => return Err(format!("comparaison inconnue : {other}")),
        },
        (Out::Flag(f), Value::Bool(e)) => f == e,
        _ => false,
    };
    if !ok {
        return Err(format!("obtenu {got:?}, attendu {expected}"));
    }
    // Nombre de corrections du dictionnaire, quand le cas le demande.
    if let Some(n) = case.get("expected_corrections").and_then(Value::as_u64) {
        let applied = dictionary::apply(&dictionary_arg(args), &start, bullet_arg(args), true);
        if applied.count as u64 != n {
            return Err(format!("corrections : {} au lieu de {n}", applied.count));
        }
    }
    Ok(Some(()))
}

#[test]
fn reference_cases() {
    let mut files: Vec<_> = std::fs::read_dir(fixtures_dir()).expect("dossier fixtures").filter_map(Result::ok).map(|e| e.path()).collect();
    files.retain(|p| p.extension().is_some_and(|e| e == "json"));
    files.sort();
    let mut failures = Vec::new();
    let mut summary: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for path in files {
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let suite = doc["suite"].as_str().unwrap().to_string();
        for case in doc["cases"].as_array().unwrap() {
            let id = case["id"].as_str().unwrap();
            let entry = summary.entry(suite.clone()).or_default();
            match run_case(case) {
                Ok(Some(())) => entry.0 += 1,
                Ok(None) => entry.1 += 1,
                Err(why) => failures.push(format!("{id} ({}) : {why}", case["name"].as_str().unwrap_or(""))),
            }
        }
    }
    println!("\n{:<26} {:>7} {:>9}", "suite", "passent", "en attente");
    for (suite, (ok, pending)) in &summary {
        println!("{suite:<26} {ok:>7} {pending:>9}");
    }
    assert!(failures.is_empty(), "{} cas en échec :\n{}", failures.len(), failures.join("\n"));
}
