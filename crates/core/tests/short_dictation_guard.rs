//! Garde-fou des dictées courtes : aucun mot perdu (23/11, « Aïku est-il adapté ? » → « Haïku adapté ? »).
use scriibe_core::cleaner::accepts;
use scriibe_core::modes::Fidelity;

#[test]
fn short_dictation_losing_a_word_is_refused() {
    assert!(!accepts("Aïku est-il adapté ?", "Haïku adapté ?", Fidelity::Light));
    assert!(!accepts("Aïku est-il adapté ?", "Est-Aïku adapté ?", Fidelity::Light));
}

#[test]
fn short_dictation_keeping_its_words_is_accepted() {
    assert!(accepts("Aïku est-il adapté ?", "Aïku est-il adapté ?", Fidelity::Light));
    assert!(accepts("Scrib a écrit littéralement la commande", "Scriibe a écrit littéralement la commande.", Fidelity::Light));
    // Une hésitation retirée n'est pas un mot perdu.
    assert!(accepts("euh je veux partir", "Je veux partir.", Fidelity::Light));
}
