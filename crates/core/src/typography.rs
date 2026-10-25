//! Typographie française : espace insécable avant `? ! ;` et `:`, et dans les heures.

use crate::re::regex;

const NBSP: &str = "\u{00A0}";

// Seulement en fin de mot (espace, fin, guillemet, autre ponctuation) : pas dans « page?id=3 » ni « ;) ».
regex!(BEFORE_SIGN, r#"(?<=[\p{L}\p{N}\)»"'’])[ \x{00A0}\x{202F}]*([?!;])(?=\s|$|[»"’?!.…])"#);
regex!(BEFORE_COLON, r#"(?<=[\p{L}\)»"'’])[ \x{00A0}\x{202F}]*:(?=\s|$)"#);
// Heures : un nombre de 0 à 24 seul, puis « h » seul ou suivi des minutes (pas « 2 hommes », ni « 100h »),
// hors noms de fichiers, options et mots-dièse (« rapport_19h.pdf », « t=1h », « #24h », « UTC+2h »).
regex!(
    HOUR_MINUTES,
    r"(?<![\p{L}\p{N}_/#=@+.])([01]?\d|2[0-4])[ \x{00A0}\x{202F}]?[hH][ \x{00A0}\x{202F}]?([0-5]\d)(?![\p{L}\p{N}_]|\.\p{L})"
);
regex!(HOUR, r"(?<![\p{L}\p{N}_/#=@+.])([01]?\d|2[0-4])[ \x{00A0}\x{202F}]?[hH](?![\p{L}\p{N}_]|\.\p{L})");

/// Typographie française : espace insécable avant `? ! ;` et avant `:` (sauf heures « 10:30 » et liens « https: »),
/// et dans les heures de Whisper (« 19h » → « 19 h », « 19h30 » → « 19 h 30 »), comme les écrit le modèle.
pub fn french_typography(text: &str) -> String {
    let t = BEFORE_SIGN.replace_all(text, format!("{NBSP}$1").as_str());
    let t = BEFORE_COLON.replace_all(&t, format!("{NBSP}:").as_str());
    let t = HOUR_MINUTES.replace_all(&t, format!("$1{NBSP}h{NBSP}$2").as_str());
    HOUR.replace_all(&t, format!("$1{NBSP}h").as_str()).into_owned()
}
