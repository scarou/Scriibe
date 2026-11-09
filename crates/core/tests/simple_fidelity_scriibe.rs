//! Garde-fou du mode Simple sur les dictées longues (S19) : cas tirés de l'historique de Christophe (09/11).
use scriibe_core::cleaner::accepts;

#[test]
fn long_dictation_changed_in_meaning_is_rejected() {
    let raw = "Par exemple, le projet pilote PC avait été réalisé sous la supervision, il me semble, sous la commande slash supervisé et j'ai donc rajouté les fonctionnalités depuis une session";
    let out = "Par exemple, le projet pilote-pc avait été réalisé sous la supervision. Je l'ai ensuite ajouté aux fonctionnalités depuis une session.";
    assert!(!accepts(raw, out, false));
}

#[test]
fn dictated_signs_do_not_count_as_lost_words() {
    let raw = "Voici ma liste de courses, deux points, retour à la ligne, puce du pain, retour à la ligne, puce des légumes, retour à la ligne, puce du lait";
    let out = "Voici ma liste de courses :\n• du pain\n• des légumes\n• du lait";
    assert!(accepts(raw, out, false));
}

#[test]
fn short_dictation_keeps_the_old_threshold() {
    assert!(accepts("Oui, écrit la revue et puis range l'inbox", "Oui, écris la revue et ensuite range l'inbox.", false));
}

#[test]
fn long_faithful_dictation_passes() {
    let raw = "alors je pense qu'on devrait vérifier le dossier de projet avant de lancer la compilation sur le poste windows";
    let out = "Alors je pense qu'on devrait vérifier le dossier de projet avant de lancer la compilation sur le poste Windows.";
    assert!(accepts(raw, out, false));
}
