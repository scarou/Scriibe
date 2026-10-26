//! Rejoue les cas de référence de `fixtures/` (tirés des vérifications de Scribe) contre le cœur Rust.
//!
//! Chaque cas a une fonction `fn` ; celles que le cœur ne sait pas encore faire sont listées dans `PENDING` : le test échoue
//! si un cas utilise une fonction qui n'est ni implémentée ni déclarée en attente, pour qu'aucun cas ne soit ignoré en silence.

use chrono::{NaiveDateTime, Utc};
use scriibe_core::dictionary::{self, Correction, UserDictionary};
use scriibe_core::history::{self, retention_calc, Calendar, DayTotal, Dictation};
use scriibe_core::hotkey::{HotkeyMonitor, Key};
use scriibe_core::lang::LinguaGuesser;
use scriibe_core::media_pause::{self, AudioProcess, Decision};
use scriibe_core::microphone::{self, MicLevel, Microphone};
use scriibe_core::modes::RewriteMode;
use scriibe_core::settings::AppSettings;
use scriibe_core::settings::Hotkey;
use scriibe_core::spoken_signs::{self, ListBullet};
use scriibe_core::translation::TranslationLanguage;
use scriibe_core::{cleaner, text, typography};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

/// Suites que ce banc d'essai ne rejoue pas : testées ailleurs ou pas encore.
fn category(suite: &str) -> Option<&'static str> {
    match suite {
        "local_engines" => Some("moteurs"),
        "ui_logic" => Some("interface"),
        "playful_schtroumpf" => Some("reporté"),
        _ => None,
    }
}

/// Les cas dont l'énoncé est un scénario ou une propriété : un test écrit à la main doit les citer par leur identifiant.
const MANUAL: &[&str] = &["invariant", "history_scenario"];

fn manual_tests_source() -> String {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut all = String::new();
    for entry in std::fs::read_dir(dir).unwrap().filter_map(Result::ok) {
        let path = entry.path();
        if path.file_name().is_some_and(|n| n != "fixtures.rs") && path.extension().is_some_and(|e| e == "rs") {
            all.push_str(&std::fs::read_to_string(path).unwrap());
        }
    }
    all
}

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

/// Navigue dans une valeur JSON : `a.b`, `a[0].b`, et `.count` pour la taille d'un tableau. Un champ absent vaut `null`.
fn field(value: &Value, path: &str) -> Value {
    let mut current = value.clone();
    for part in path.split('.') {
        let (name, index) = match part.split_once('[') {
            Some((n, rest)) => (n, rest.trim_end_matches(']').parse::<usize>().ok()),
            None => (part, None),
        };
        if name == "count" && current.is_array() {
            return Value::from(current.as_array().map_or(0, Vec::len));
        }
        if !name.is_empty() {
            current = current.get(name).cloned().unwrap_or(Value::Null);
        }
        if let Some(i) = index {
            current = current.get(i).cloned().unwrap_or(Value::Null);
        }
    }
    current
}

fn same_json(got: &Value, want: &Value) -> bool {
    match (got.as_f64(), want.as_f64()) {
        (Some(a), Some(b)) => (a - b).abs() < 1e-9,
        _ => got == want,
    }
}

fn settings_from(args: &Value) -> AppSettings {
    AppSettings::from_value(args).unwrap_or_default()
}

/// Propriétés calculées des réglages, nommées comme dans Scribe.
fn settings_view(settings: &AppSettings) -> Value {
    let mut view = serde_json::to_value(settings).unwrap();
    let o = view.as_object_mut().unwrap();
    o.insert("usesCleanupModel".into(), settings.uses_cleanup_model().into());
    o.insert("skipApplies".into(), settings.skip_applies().into());
    o.insert("listsOn".into(), settings.lists_on().into());
    o.insert("rewritingOn".into(), settings.rewriting_on().into());
    o.insert("hasCleanupEngine".into(), settings.has_cleanup_engine().into());
    o.insert("activeCleanupModel".into(), settings.active_cleanup_model().into());
    o.insert("everythingLocal".into(), settings.everything_local().into());
    o.insert("activeCustomMode".into(), serde_json::to_value(settings.active_custom_mode()).unwrap());
    view
}

fn check_fields(view: &Value, expected: &Value) -> Result<(), String> {
    for (path, want) in expected.as_object().ok_or("attendu : un dictionnaire")? {
        let got = field(view, path);
        if !same_json(&got, want) {
            return Err(format!("{path} : obtenu {got}, attendu {want}"));
        }
    }
    Ok(())
}

fn language_arg(args: &Value) -> TranslationLanguage {
    serde_json::from_value(args["language"].clone()).unwrap_or_default()
}

/// Les cas qui ne sont pas « du texte entrant, du texte sortant ». `None` : ce n'est pas un cas de cette sorte.
fn run_special(case: &Value) -> Option<Result<(), String>> {
    let name = case["fn"].as_str()?;
    let args = &case["args"];
    let expected = &case["expected"];
    let flag = |got: bool| -> Result<(), String> {
        if Some(got) == expected.as_bool() {
            Ok(())
        } else {
            Err(format!("obtenu {got}, attendu {expected}"))
        }
    };
    let text_is = |got: String| -> Result<(), String> {
        if Some(got.as_str()) == expected.as_str() {
            Ok(())
        } else {
            Err(format!("obtenu {got:?}, attendu {expected}"))
        }
    };
    Some(match name {
        "cleaner_accepts" => {
            let mode = args["mode"].as_str().and_then(RewriteMode::from_raw).unwrap_or(RewriteMode::Simple);
            flag(cleaner::accepts_for_mode(args["input"].as_str()?, args["output"].as_str()?, mode))
        }
        "cleaner_strip" => text_is(cleaner::strip(args["text"].as_str()?)),
        "accepts_translation" => {
            flag(cleaner::accepts_translation(args["input"].as_str()?, args["output"].as_str()?, language_arg(args), guesser()))
        }
        "skips_cleanup" => {
            let settings = settings_from(&args["settings"]);
            flag(cleaner::skips_cleanup(args["text"].as_str()?, &settings, args["translating"].as_bool()?))
        }
        "decode_settings" => match AppSettings::from_json(args["json"].as_str()?) {
            Ok(settings) => check_fields(&settings_view(&settings), expected),
            Err(e) => Err(format!("réglages illisibles : {e}")),
        },
        "settings_derived" => check_fields(&settings_view(&settings_from(&args["settings"])), expected),
        "settings_roundtrip" => {
            let mut settings = settings_from(&args["settings"]);
            if args["settings"]["chosen"] == "customModes[0]" {
                let id = settings.custom_modes.first()?.id.clone();
                settings.set_mode_choice(&id);
            }
            let json = serde_json::to_string(&settings).unwrap();
            match AppSettings::from_json(&json) {
                Ok(back) => check_fields(&settings_view(&back), expected),
                Err(e) => Err(format!("relecture impossible : {e}")),
            }
        }
        "bench_contains" => flag(scriibe_core::bench::contains(
            args["form"].as_str()?,
            args["text"].as_str()?,
            args["case_sensitive"].as_bool().unwrap_or(false),
        )),
        "bench_checks_pass" => {
            let phrase = &scriibe_core::bench::phrases()[args["phrase"].as_u64()? as usize];
            let text = if args["as_read"].as_bool() == Some(true) { phrase.text.as_str() } else { args["text"].as_str()? };
            flag(phrase.failures(text).is_empty())
        }
        "prompt_fingerprint" => text_is(cleaner::built_in_prompts_fingerprint()),
        "hotkey_sequence" => hotkey_sequence(args, expected, case.get("comparison").and_then(Value::as_str)),
        "command_held" => flag(hotkey_of(args).command_held(&keys_of(&args["held"]))),
        "media_decision" => media_decision(args, expected),
        "media_new_players" => {
            let known: std::collections::HashSet<i64> = args["known"].as_array()?.iter().filter_map(Value::as_i64).collect();
            let got =
                media_pause::new_players(&processes(&args["processes"]), args["own_pid"].as_i64()?, &known, &media_pause::IGNORED_MACOS);
            if serde_json::to_value(&got).unwrap() == *expected {
                Ok(())
            } else {
                Err(format!("obtenu {got:?}, attendu {expected}"))
            }
        }
        "mic_level" => {
            let got = format!("{:?}", MicLevel::assess(args["rms"].as_f64()? as f32)).to_lowercase();
            text_is(got)
        }
        "mic_resolve" => {
            let list: Vec<Microphone> = args["available"].as_array()?.iter().map(microphone).collect();
            let r = microphone::resolve(args["uid"].as_str()?, &list);
            let view = serde_json::json!({ "microphone": r.microphone.map(|m| serde_json::json!({"uid": m.uid})), "missing": r.missing });
            check_fields(&view, expected)
        }
        "mic_label" => {
            let label = microphone(&args["microphone"]).label();
            match case.get("comparison").and_then(Value::as_str) {
                Some("contains") if label.contains(expected.as_str()?) => Ok(()),
                None if Some(label.as_str()) == expected.as_str() => Ok(()),
                _ => Err(format!("libellé {label:?}, attendu {expected}")),
            }
        }
        "decode_history" => match history::lossy_list(args["json"].as_str()?, Dictation::from_value) {
            Ok(list) => check_fields(&serde_json::to_value(&list).unwrap(), expected),
            Err(e) => Err(e.to_string()),
        },
        "decode_history_entry" => {
            let value: Value = serde_json::from_str(args["json"].as_str()?).unwrap();
            let entry = Dictation::from_value(&value);
            let view = serde_json::json!({ "decoded": entry.is_some(), "timing": entry.and_then(|e| e.timing).map(|t| t.total) });
            check_fields(&view, expected)
        }
        "decode_dictionary" => match UserDictionary::from_json(args["json"].as_str()?) {
            Ok(d) => check_fields(&serde_json::json!({ "corrections": d.corrections }), expected),
            Err(e) => Err(e.to_string()),
        },
        "decode_day_totals" => match history::lossy_list(args["json"].as_str()?, DayTotal::from_value) {
            Ok(list) => check_fields(&serde_json::to_value(&list).unwrap(), expected),
            Err(e) => Err(e.to_string()),
        },
        "history_roundtrip" => {
            let mut entry = args["entry"].clone();
            entry["date"] = "2026-01-01T00:00:00Z".into();
            let dictation = Dictation::from_value(&entry)?;
            let back = Dictation::from_value(&serde_json::to_value(&dictation).unwrap())?;
            check_fields(&serde_json::to_value(&back).unwrap(), expected)
        }
        "retention_cutoff" | "retention_cutoff_equals" => retention(name, args, expected),
        _ => return None,
    })
}

fn hotkey_of(args: &Value) -> Hotkey {
    match args["hotkey"].as_str() {
        Some("right_command") => Hotkey::RightCommand,
        Some("left_option") => Hotkey::LeftOption,
        Some("fn") => Hotkey::Fn,
        _ => Hotkey::RightOption,
    }
}

fn keys_of(v: &Value) -> Vec<Key> {
    v.as_array().map(|a| a.iter().filter_map(|k| k.as_str().and_then(Key::from_name)).collect()).unwrap_or_default()
}

fn hotkey_sequence(args: &Value, expected: &Value, comparison: Option<&str>) -> Result<(), String> {
    let mut monitor = HotkeyMonitor::new(hotkey_of(args));
    let mut log = String::new();
    let typed_after = args.get("typed_after_step").and_then(Value::as_u64);
    for (i, step) in args["steps"].as_array().ok_or("étapes")?.iter().enumerate() {
        let key = step["key"].as_str().and_then(Key::from_name).ok_or("touche inconnue")?;
        log.extend(monitor.handle(key, &keys_of(&step["held"])).into_iter().map(|e| e.letter()));
        if typed_after == Some(i as u64) {
            log.extend(monitor.handle_key_down().into_iter().map(|e| e.letter()));
        }
    }
    let want = expected.as_str().ok_or("attendu")?;
    let ok = if comparison == Some("prefix") { log.starts_with(want) } else { log == want };
    if ok {
        Ok(())
    } else {
        Err(format!("journal {log:?}, attendu {want:?}"))
    }
}

fn processes(v: &Value) -> Vec<AudioProcess> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|p| AudioProcess {
                    pid: p["pid"].as_i64().unwrap_or(0),
                    bundle: p["bundle"].as_str().unwrap_or("").into(),
                    output: p["output"].as_bool().unwrap_or(false),
                    input: p["input"].as_bool().unwrap_or(false),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn media_decision(args: &Value, expected: &Value) -> Result<(), String> {
    let got = media_pause::decision(&processes(&args["processes"]), args["own_pid"].as_i64().ok_or("pid")?, &media_pause::IGNORED_MACOS);
    let want = match expected {
        Value::String(s) if s == "nothing_playing" => Decision::NothingPlaying,
        Value::String(s) if s == "in_call" => Decision::InCall,
        v => Decision::Play(v["play"].as_array().ok_or("play")?.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()),
    };
    if got == want {
        Ok(())
    } else {
        Err(format!("obtenu {got:?}, attendu {want:?}"))
    }
}

fn microphone(v: &Value) -> Microphone {
    Microphone {
        id: v["id"].as_i64().unwrap_or(0),
        uid: v["uid"].as_str().unwrap_or("").into(),
        name: v["name"].as_str().unwrap_or("").into(),
        bluetooth: v["bluetooth"].as_bool().unwrap_or(false),
    }
}

fn local_time(text: &str) -> chrono::DateTime<Utc> {
    NaiveDateTime::parse_from_str(&text[..19], "%Y-%m-%dT%H:%M:%S").unwrap().and_utc()
}

fn retention(name: &str, args: &Value, expected: &Value) -> Result<(), String> {
    let cal = Calendar::new(Utc);
    let now = local_time(args["now"].as_str().ok_or("now")?);
    let cutoff = retention_calc::cutoff(&cal, args["keep_days"].as_i64().ok_or("jours")?, now);
    if name == "retention_cutoff" {
        let got = format!("{} (heure locale)", cutoff.format("%Y-%m-%dT%H:%M:%S"));
        return if Some(got.as_str()) == expected.as_str() { Ok(()) } else { Err(format!("obtenu {got}, attendu {expected}")) };
    }
    let other = retention_calc::cutoff(&cal, args["equals_keep_days"].as_i64().ok_or("jours")?, now);
    if (cutoff == other) == expected.as_bool().unwrap_or(true) {
        Ok(())
    } else {
        Err("coupures différentes".into())
    }
}

fn guesser() -> &'static LinguaGuesser {
    static G: std::sync::OnceLock<LinguaGuesser> = std::sync::OnceLock::new();
    G.get_or_init(LinguaGuesser::default)
}

fn run_case(case: &Value) -> Result<Option<()>, String> {
    if let Some(done) = run_special(case) {
        return done.map(Some);
    }
    let fns: Vec<String> = match &case["fn"] {
        Value::String(s) => vec![s.clone()],
        Value::Array(a) => a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
        _ => return Err("fn illisible".into()),
    };
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
    let manual_source = manual_tests_source().replace('_', "-");
    let mut failures = Vec::new();
    let mut summary: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for path in files {
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let suite = doc["suite"].as_str().unwrap().to_string();
        for case in doc["cases"].as_array().unwrap() {
            let id = case["id"].as_str().unwrap();
            let entry = summary.entry(suite.clone()).or_default();
            if category(&suite).is_some() {
                entry.1 += 1;
                continue;
            }
            let manual = case["fn"].as_str().is_some_and(|f| MANUAL.contains(&f));
            if manual {
                if manual_source.contains(&id.replace('_', "-")) {
                    entry.0 += 1;
                } else {
                    failures.push(format!("{id} ({}) : aucun test écrit à la main ne cite ce cas", case["name"].as_str().unwrap_or("")));
                }
                continue;
            }
            match run_case(case) {
                Ok(Some(())) => entry.0 += 1,
                Ok(None) => entry.1 += 1,
                Err(why) => failures.push(format!("{id} ({}) : {why}", case["name"].as_str().unwrap_or(""))),
            }
        }
    }
    println!("\n{:<26} {:>7}  ailleurs", "suite", "passent");
    for (suite, (ok, elsewhere)) in &summary {
        println!("{suite:<26} {ok:>7}  {elsewhere:>3} {}", category(suite).unwrap_or(""));
    }
    assert!(failures.is_empty(), "{} cas en échec :\n{}", failures.len(), failures.join("\n"));
}
