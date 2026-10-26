//! Clés d'API : variables d'environnement (tous les systèmes). Le gestionnaire d'identifiants du système s'y ajoute (voir `windows`).
//! Les clés ne sont jamais écrites dans le journal ni affichées.

use scriibe_engines::runtime::{KeyKind, Secrets};

pub fn env_name(kind: KeyKind) -> &'static str {
    match kind {
        KeyKind::Deepgram => "DEEPGRAM_API_KEY",
        KeyKind::OpenRouter => "OPENROUTER_API_KEY",
        KeyKind::GitHub => "GITHUB_TOKEN",
    }
}

/// Nom de la clé dans le gestionnaire d'identifiants (et son compte, sous macOS).
pub fn account_name(kind: KeyKind) -> &'static str {
    match kind {
        KeyKind::Deepgram => "deepgram",
        KeyKind::OpenRouter => "openrouter",
        KeyKind::GitHub => "github",
    }
}

fn non_empty(value: String) -> Option<String> {
    let value = value.trim().to_string();
    (!value.is_empty()).then_some(value)
}

pub struct EnvSecrets;

impl Secrets for EnvSecrets {
    fn get(&self, kind: KeyKind) -> Option<String> {
        std::env::var(env_name(kind)).ok().and_then(non_empty)
    }
}

/// Cherche dans plusieurs sources, la première qui répond gagne.
pub struct ChainedSecrets(pub Vec<Box<dyn Secrets>>);

impl Secrets for ChainedSecrets {
    fn get(&self, kind: KeyKind) -> Option<String> {
        self.0.iter().find_map(|s| s.get(kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed(Option<&'static str>);
    impl Secrets for Fixed {
        fn get(&self, _: KeyKind) -> Option<String> {
            self.0.map(str::to_string)
        }
    }

    #[test]
    fn the_first_source_that_answers_wins() {
        let chain = ChainedSecrets(vec![Box::new(Fixed(None)), Box::new(Fixed(Some("b"))), Box::new(Fixed(Some("c")))]);
        assert_eq!(chain.get(KeyKind::Deepgram).as_deref(), Some("b"));
        assert_eq!(ChainedSecrets(vec![]).get(KeyKind::OpenRouter), None);
    }

    #[test]
    fn blank_values_do_not_count() {
        assert_eq!(non_empty("  ".into()), None);
        assert_eq!(non_empty(" k \n".into()).as_deref(), Some("k"));
    }
}
