//! Rejoue les cas de `fixtures/local_engines.json` (tirés des vérifications de Scribe) contre les moteurs locaux.

use scriibe_core::settings::CleanupEngine;
use scriibe_engines::fs::FileSystem;
use scriibe_engines::{llama, whisper};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Des fichiers en mémoire, sous `/fake`.
struct FakeFs {
    files: Vec<PathBuf>,
}

impl FakeFs {
    fn relative(files: &[Value]) -> Self {
        Self { files: files.iter().filter_map(|f| f.as_str()).map(|f| Path::new("/fake").join(f)).collect() }
    }

    fn absolute(files: Vec<String>) -> Self {
        Self { files: files.into_iter().map(PathBuf::from).collect() }
    }
}

impl FileSystem for FakeFs {
    fn exists(&self, path: &Path) -> bool {
        self.files.iter().any(|f| f == path)
    }

    fn list_dir(&self, dir: &Path) -> Vec<String> {
        self.files.iter().filter(|f| f.parent() == Some(dir)).filter_map(|f| f.file_name()?.to_str().map(str::to_string)).collect()
    }

    fn walk(&self, root: &Path) -> Vec<PathBuf> {
        self.files.iter().filter(|f| f.starts_with(root)).cloned().collect()
    }
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

fn expect<T: PartialEq + std::fmt::Debug>(got: T, want: T) -> Result<(), String> {
    if got == want {
        Ok(())
    } else {
        Err(format!("obtenu {got:?}, attendu {want:?}"))
    }
}

fn run(case: &Value) -> Result<(), String> {
    let args = &case["args"];
    let want = &case["expected"];
    match case["fn"].as_str().ok_or("fn")? {
        "is_whisper_model" => expect(whisper::is_whisper_model(args["path"].as_str().ok_or("path")?), want.as_bool().ok_or("bool")?),
        "distinct_models" | "same_model" => {
            let sizes: HashMap<String, u64> = args["sizes"]
                .as_object()
                .map(|o| o.iter().filter_map(|(k, v)| Some((k.clone(), v.as_u64()?))).collect())
                .unwrap_or_default();
            let size = |p: &str| sizes.get(p).copied();
            if case["fn"] == "distinct_models" {
                expect(whisper::distinct(&strings(&args["paths"]), size), strings(want))
            } else {
                expect(
                    whisper::same_model(args["a"].as_str().ok_or("a")?, args["b"].as_str().ok_or("b")?, size),
                    want.as_bool().ok_or("bool")?,
                )
            }
        }
        "whisper_should_release" => expect(
            whisper::should_release(
                args["idle_seconds"].as_f64().ok_or("idle")?,
                args["limit_minutes"].as_i64().ok_or("limit")?,
                args["busy"].as_bool().ok_or("busy")?,
            ),
            want.as_bool().ok_or("bool")?,
        ),
        "llama_base_name" => expect(llama::base_name(args["file"].as_str().ok_or("file")?), want.as_str().ok_or("str")?.to_string()),
        "llama_arguments" => {
            let assistant = args["assistant"].as_str();
            let got = llama::arguments(args["model"].as_str().ok_or("model")?, assistant, args["port"].as_u64().ok_or("port")? as u16);
            let start = strings(&want["starts_with"]);
            if got.len() < start.len() || got[..start.len()] != start[..] {
                return Err(format!("ne commence pas par {start:?} : {got:?}"));
            }
            let joined = got.join(" ");
            for part in strings(&want["contains"]) {
                if !joined.contains(&part) {
                    return Err(format!("{part:?} absent de {joined}"));
                }
            }
            for part in strings(&want["not_contains"]) {
                if got.contains(&part) {
                    return Err(format!("{part:?} présent dans {joined}"));
                }
            }
            Ok(())
        }
        "llama_detect_models" => {
            let fs = FakeFs::relative(args["files"].as_array().ok_or("files")?);
            let found = llama::detect_models(&fs, &[PathBuf::from("/fake")], None);
            let names: Vec<String> = found.iter().map(|p| scriibe_engines::file_name(p).to_string()).collect();
            if let Some(expected_names) = want.get("file_names") {
                return expect(names, strings(expected_names));
            }
            expect(names.len() as u64, want["count"].as_u64().ok_or("count")?)?;
            expect(names.first().cloned().unwrap_or_default(), want["first_file_name"].as_str().ok_or("first")?.to_string())?;
            let prefix = want["none_starts_with"].as_str().ok_or("prefix")?;
            if names.iter().any(|n| n.starts_with(prefix)) {
                Err(format!("un fichier commence par {prefix} : {names:?}"))
            } else {
                Ok(())
            }
        }
        "llama_assistant" => {
            let fs = FakeFs::relative(args["files"].as_array().ok_or("files")?);
            for (model, wanted) in want.as_object().ok_or("map")? {
                // Sous Windows, les chemins mélangent « / » et « \\ » : on compare avec un seul séparateur.
                let normal = |p: String| p.replace('\\', "/");
                let got = llama::assistant(&fs, &Path::new("/fake").join(model).to_string_lossy()).map(normal);
                let wanted = wanted.as_str().map(|w| normal(Path::new("/fake").join(w).to_string_lossy().into_owned()));
                expect(got, wanted).map_err(|e| format!("{model} : {e}"))?;
            }
            Ok(())
        }
        "llama_status" => {
            let models = strings(&args["models"]);
            let chosen = args["chosen"].as_str().ok_or("chosen")?;
            let mut files = models.clone();
            if args["assistant_exists"].as_bool().unwrap_or(false) {
                let dir = Path::new(chosen).parent().ok_or("dossier")?;
                let base = llama::base_name(scriibe_engines::file_name(chosen));
                files.push(dir.join(format!("mtp-{base}-Q8_0.gguf")).to_string_lossy().into_owned());
            }
            let text = llama::status(&FakeFs::absolute(files), args["binary"].as_str(), &models, chosen, "brew install llama.cpp");
            for part in strings(want) {
                if !text.contains(&part) {
                    return Err(format!("{part:?} absent de {text:?}"));
                }
            }
            Ok(())
        }
        "cleanup_engine_display" => {
            let engine: CleanupEngine = serde_json::from_value(args["engine"].clone()).map_err(|e| e.to_string())?;
            let model = args["model"].as_str().ok_or("model")?;
            for (key, value) in want.as_object().ok_or("fields")? {
                let got: Value = match key.as_str() {
                    "isLocal" => engine.is_local().into(),
                    "name" => engine.name().into(),
                    "displayModel" => engine.display_model(model).into(),
                    "port" => llama::PORT.into(),
                    other => return Err(format!("champ inconnu : {other}")),
                };
                expect(&got, value)?;
            }
            Ok(())
        }
        "invariant" => Ok(()), // vérifié par `required_options_are_known` ci-dessous
        other => Err(format!("fonction inconnue : {other}")),
    }
}

#[test]
fn reference_cases() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/local_engines.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut failures = Vec::new();
    let mut count = 0;
    for case in doc["cases"].as_array().unwrap() {
        count += 1;
        if let Err(why) = run(case) {
            failures.push(format!("{} ({}) : {why}", case["id"], case["name"].as_str().unwrap_or("")));
        }
    }
    println!("local_engines : {count} cas rejoués");
    assert!(failures.is_empty(), "{} cas en échec :\n{}", failures.len(), failures.join("\n"));
}

/// local_engines-039 : le `llama-server` installé connaît les options utilisées (une mise à jour de llama.cpp peut en retirer une).
/// Sans `llama-server` installé, le test ne vérifie rien.
#[test]
fn local_engines_039_required_options_are_known() {
    let Some(binary) =
        std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join("llama-server")).find(|f| f.is_file()))
    else {
        eprintln!("llama-server absent du PATH : options non vérifiées");
        return;
    };
    let out = std::process::Command::new(binary).arg("--help").output().expect("llama-server --help");
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let missing: Vec<_> = llama::REQUIRED_OPTIONS.iter().filter(|o| !text.contains(**o)).collect();
    assert!(missing.is_empty(), "options absentes : {missing:?}");
}
