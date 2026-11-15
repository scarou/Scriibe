//! Recherches vocales (chantier 6, étape 1b) : « nouvelle recherche <site> … », **sous la touche de commande seulement**.
//!
//! Couche pure : on ne dit ici que **quelle** adresse s'ouvrirait, jamais comment (l'ouverture est dans `crates/platform`, `open.rs`).
//! Quatre sites livrés (Google, YouTube, Wikipédia, DuckDuckGo) et ceux de la personne (`webSearches`).
//!
//! **Sécurité** (décision 4 de `docs/CADRAGE_COMMANDES.md`, défaut B4). La voix ne doit jamais choisir l'hôte de l'adresse ouverte. Un
//! modèle d'adresse n'est donc valable que s'il est en `https`, à hôte **fixe** (ni `{q}` ni `@` dans l'autorité), avec un seul `{q}`
//! dans le chemin ou la requête, en caractères ASCII d'adresse seulement. Le contrôle a lieu **à la lecture des réglages**
//! ([`WebSearch::cleaned`]) comme à la page, puis une dernière fois sur l'adresse finale ([`search_url`] : l'hôte final est celui du modèle).
//! Ce qui est dit n'entre dans l'adresse qu'encodé (RFC 3986) : il ne peut rien y ajouter (`/`, `@`, `#`, `?`, `&`).

use serde::{Deserialize, Serialize};

use crate::commands::{normalize, Argument, CommandDef, Lang, Needs, Scope};
use crate::import_commands::{MAX_PHRASE, MAX_PHRASES};
use crate::settings::AppSettings;

/// Préfixe des identifiants des recherches : `recherche.<n>`. Le journal ne garde que cet identifiant (jamais le nom du site, ni la requête).
pub const PREFIX: &str = "recherche.";
/// Identifiant que la page donne à un site pas encore créé, pour contrôler sa première phrase (`commands::validate_phrase`).
pub const NEW: &str = "recherche.nouveau";
/// Les numéros 1 à 9 sont aux sites livrés ; ceux de la personne commencent ici.
pub const FIRST_CUSTOM: u32 = 10;
/// Nombre maximal de sites de la personne.
pub const MAX_SITES: usize = 20;
/// Longueur maximale (en caractères) du nom d'un site.
pub const MAX_NAME: usize = 40;
/// Longueur maximale (en caractères) d'un modèle d'adresse.
pub const MAX_TEMPLATE: usize = 300;
/// Au plus ce nombre de caractères de la requête dite vont dans l'adresse (le reste est coupé).
pub const MAX_QUERY_CHARS: usize = 300;
/// Longueur maximale de l'adresse finale (en octets) : au-delà, elle n'est pas ouverte.
pub const MAX_URL: usize = 2000;
/// L'endroit du modèle où la requête dite est mise.
pub const PLACEHOLDER: &str = "{q}";

/// Un site de recherche de la personne : un nom, un modèle d'adresse avec `{q}`, des phrases (« nouvelle recherche amazon »).
/// Les phrases valent pour toutes les langues de dictée (c'est la personne qui les écrit).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearch {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub phrases: Vec<String>,
}

/// Pourquoi un modèle d'adresse est refusé. Le texte est fixe (il va à la page, jamais dans le journal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateError {
    /// Pas une adresse en `https://` (ni `http:`, ni `javascript:`, ni autre chose), ou trop longue.
    NotHttps,
    /// Un caractère qui n'est pas permis dans une adresse (espace, guillemet, `<`, `>`, `\`, accolade en trop, non ASCII…).
    BadCharacter,
    /// Pas exactement un `{q}`.
    Placeholder,
    /// L'hôte n'est pas fixe ou pas un nom de site (`{q}` ou `@` avant le premier `/`, adresse numérique, nom sans point…).
    Host,
}

impl TemplateError {
    pub fn reason(self) -> &'static str {
        match self {
            Self::NotHttps => "L'adresse doit commencer par https:// et ne pas dépasser 300 caractères",
            Self::BadCharacter => "L'adresse contient un caractère interdit (espace, guillemet, < > \\ ou accolade)",
            Self::Placeholder => "L'adresse doit contenir {q} une seule fois, là où va le texte cherché",
            Self::Host => "Le nom du site doit être fixe : {q} ne peut pas être avant le premier /, ni de @ dans l'adresse",
        }
    }
}

/// Le préfixe `https://` (sans tenir compte de la casse) : renvoie l'autorité (hôte et port) puis le reste (chemin, requête).
fn split_https(url: &str) -> Option<(&str, &str)> {
    let head = url.get(..8)?;
    if !head.eq_ignore_ascii_case("https://") {
        return None;
    }
    let after = &url[8..];
    let end = after.find(['/', '?', '#']).unwrap_or(after.len());
    Some((&after[..end], &after[end..]))
}

/// L'hôte d'une autorité (en minuscules), s'il est valable : des étiquettes de lettres, chiffres et tirets, au moins un point, pas d'adresse
/// numérique, pas de `@` (identifiant), port facultatif. `None` sinon.
fn host_of(authority: &str) -> Option<String> {
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    if let Some(port) = port {
        if port.is_empty() || port.len() > 5 || !port.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
    }
    let host = host.to_ascii_lowercase();
    if host.is_empty() || host.len() > 253 || !host.contains('.') {
        return None;
    }
    let labels: Vec<&str> = host.split('.').collect();
    let ok = labels.iter().all(|l| {
        !l.is_empty()
            && l.len() <= 63
            && !l.starts_with('-')
            && !l.ends_with('-')
            && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    });
    // Une adresse numérique (192.168.0.1) n'est pas un nom de site.
    let numeric_end = labels.last().is_some_and(|l| l.bytes().all(|b| b.is_ascii_digit()));
    (ok && !numeric_end).then_some(host)
}

/// Contrôle un modèle d'adresse et rend son hôte (en minuscules), ou la raison du refus.
pub fn check_template(template: &str) -> Result<String, TemplateError> {
    if template.chars().count() > MAX_TEMPLATE {
        return Err(TemplateError::NotHttps);
    }
    let Some((authority, rest)) = split_https(template) else { return Err(TemplateError::NotHttps) };
    // Que des caractères ASCII d'adresse : ni espace ni contrôle, ni guillemet, `<`, `>`, `\`, `^`, `` ` ``, `|`. Les accolades ne servent
    // qu'au seul `{q}`, vérifié plus bas.
    if !template.bytes().all(|b| (0x21..=0x7E).contains(&b) && !matches!(b, b'"' | b'<' | b'>' | b'\\' | b'^' | b'`' | b'|')) {
        return Err(TemplateError::BadCharacter);
    }
    if template.matches(PLACEHOLDER).count() != 1 || template.replace(PLACEHOLDER, "").contains(['{', '}']) {
        return Err(TemplateError::Placeholder);
    }
    // Le `{q}` n'est ni dans l'autorité (l'hôte ne serait pas fixe), ni après un `#` (le site ne le recevrait pas).
    if authority.contains(PLACEHOLDER) || authority.contains('@') || authority.contains('%') {
        return Err(TemplateError::Host);
    }
    if let (Some(hash), Some(slot)) = (rest.find('#'), rest.find(PLACEHOLDER)) {
        if hash < slot {
            return Err(TemplateError::Host);
        }
    }
    host_of(authority).ok_or(TemplateError::Host)
}

/// L'hôte d'un modèle valable, pour l'afficher (liste, aperçu d'import) ; `None` si le modèle est refusé.
pub fn template_host(template: &str) -> Option<String> {
    check_template(template).ok()
}

/// Encode un morceau d'adresse (RFC 3986) : seules les lettres, chiffres et `- . _ ~` restent tels quels ; tout le reste, octet par octet.
pub fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// La requête telle qu'elle va dans l'adresse : blancs et contrôles ramenés à une espace, coupée à [`MAX_QUERY_CHARS`], encodée.
fn encoded_query(query: &str) -> String {
    let flat: String = query.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let flat = flat.split_whitespace().collect::<Vec<_>>().join(" ");
    percent_encode(&flat.chars().take(MAX_QUERY_CHARS).collect::<String>())
}

/// L'adresse à ouvrir : le modèle avec la requête encodée à la place de `{q}`. `None` si le modèle est refusé, si la requête est vide ou si
/// l'adresse finale n'a pas le même hôte que le modèle ou dépasse [`MAX_URL`] (contrôle fait **sur le résultat**).
pub fn search_url(template: &str, query: &str) -> Option<String> {
    let host = check_template(template).ok()?;
    let encoded = encoded_query(query);
    if encoded.is_empty() {
        return None;
    }
    let url = template.replace(PLACEHOLDER, &encoded);
    let (authority, _) = split_https(&url)?;
    (host_of(authority).as_deref() == Some(host.as_str()) && url.len() <= MAX_URL).then_some(url)
}

/// Une adresse peut-elle être ouverte par une commande vocale ? Seulement `https://` à hôte nommé (ni identifiant `@`, ni adresse numérique,
/// ni accolade) ou `mailto:`, en caractères ASCII d'adresse, au plus [`MAX_URL`] octets. Contrôle **commun** de la fonction d'ouverture de
/// `crates/platform` (`open::open_address`) : rien d'autre ne s'ouvre à la voix, quelle que soit la commande qui l'a construite.
pub fn is_openable(url: &str) -> bool {
    if url.len() > MAX_URL
        || !url.bytes().all(|b| (0x21..=0x7E).contains(&b) && !matches!(b, b'"' | b'<' | b'>' | b'\\' | b'^' | b'`' | b'|' | b'{' | b'}'))
    {
        return false;
    }
    if let Some((authority, _)) = split_https(url) {
        return host_of(authority).is_some();
    }
    url.get(..7).is_some_and(|scheme| scheme.eq_ignore_ascii_case("mailto:")) && url.len() > 7
}

/// Les sites livrés : numéro, modèle en français, modèle en anglais (Wikipédia change de langue), phrases françaises, phrases anglaises.
/// Les phrases comprennent les variantes de Whisper (« you tube », « wiki pédia », « duck duck go »).
struct Site {
    id: &'static str,
    fr: &'static str,
    en: &'static str,
    phrases_fr: &'static [&'static str],
    phrases_en: &'static [&'static str],
}

const SITES: [Site; 4] = [
    Site {
        id: "recherche.1",
        fr: "https://www.google.com/search?q={q}",
        en: "https://www.google.com/search?q={q}",
        phrases_fr: &["nouvelle recherche google", "nouvelle recherche sur google"],
        phrases_en: &["new search google", "new search on google"],
    },
    Site {
        id: "recherche.2",
        fr: "https://www.youtube.com/results?search_query={q}",
        en: "https://www.youtube.com/results?search_query={q}",
        phrases_fr: &[
            "nouvelle recherche youtube",
            "nouvelle recherche you tube",
            "nouvelle recherche sur youtube",
            "nouvelle recherche sur you tube",
        ],
        phrases_en: &["new search youtube", "new search you tube", "new search on youtube", "new search on you tube"],
    },
    Site {
        id: "recherche.3",
        fr: "https://fr.wikipedia.org/w/index.php?search={q}",
        en: "https://en.wikipedia.org/w/index.php?search={q}",
        phrases_fr: &[
            "nouvelle recherche wikipédia",
            "nouvelle recherche wiki pédia",
            "nouvelle recherche sur wikipédia",
            "nouvelle recherche sur wiki pédia",
        ],
        phrases_en: &["new search wikipedia", "new search wiki pedia", "new search on wikipedia", "new search on wiki pedia"],
    },
    Site {
        id: "recherche.4",
        fr: "https://duckduckgo.com/?q={q}",
        en: "https://duckduckgo.com/?q={q}",
        phrases_fr: &[
            "nouvelle recherche duckduckgo",
            "nouvelle recherche duck duck go",
            "nouvelle recherche sur duckduckgo",
            "nouvelle recherche sur duck duck go",
        ],
        phrases_en: &["new search duckduckgo", "new search duck duck go", "new search on duckduckgo", "new search on duck duck go"],
    },
];

/// Le début de toute phrase de recherche livrée : une phrase qui commence ainsi mais ne désigne aucun site connu est « Site de recherche
/// inconnu » (et non une consigne de reformulation : la recherche sur internet commence par « nouvelle recherche », décision de l'auteur).
pub const SEARCH_LEADS: [&str; 2] = ["nouvelle recherche", "new search"];

/// Les fiches des quatre sites livrés (« sous la touche seulement », un complément obligatoire : ce qu'on cherche).
pub fn builtin_defs() -> Vec<CommandDef> {
    let list = |l: &[&str]| l.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    SITES
        .iter()
        .map(|s| CommandDef {
            scope: Scope::UnderKey,
            id: s.id.into(),
            needs: Needs::Nothing,
            argument: Argument::Rest,
            phrases_fr: list(s.phrases_fr),
            phrases_en: list(s.phrases_en),
        })
        .collect()
}

/// Les fiches de tous les sites en vigueur : les livrés, puis ceux de la personne (déjà contrôlés à la lecture des réglages).
pub fn defs(settings: &AppSettings) -> Vec<CommandDef> {
    let mut all = builtin_defs();
    all.extend(settings.web_searches.iter().map(|w| CommandDef {
        scope: Scope::UnderKey,
        id: w.id.clone(),
        needs: Needs::Nothing,
        argument: Argument::Rest,
        phrases_fr: w.phrases.clone(),
        phrases_en: w.phrases.clone(),
    }));
    all
}

/// Cet identifiant est-il celui d'une recherche ?
pub fn is_search(id: &str) -> bool {
    id.strip_prefix(PREFIX).is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// Le modèle d'adresse du site `id` : celui du site livré (Wikipédia dans la langue de la phrase dite), sinon celui du site de la personne.
fn template_of<'a>(id: &str, lang: Lang, settings: &'a AppSettings) -> Option<&'a str> {
    if let Some(site) = SITES.iter().find(|s| s.id == id) {
        return Some(match lang {
            Lang::Fr => site.fr,
            Lang::En => site.en,
        });
    }
    settings.web_searches.iter().find(|w| w.id == id).map(|w| w.url.as_str())
}

/// L'adresse à ouvrir pour la recherche `id` dite en `lang` avec la requête `query`. `None` : site disparu, modèle refusé, requête vide.
pub fn target(id: &str, lang: Lang, query: &str, settings: &AppSettings) -> Option<String> {
    search_url(template_of(id, lang, settings)?, query)
}

/// Toutes les phrases (normalisées) des sites livrés, dans les deux langues : celles qu'un site de la personne ne peut pas reprendre.
fn builtin_phrases() -> Vec<String> {
    SITES.iter().flat_map(|s| s.phrases_fr.iter().chain(s.phrases_en)).map(|p| normalize(p)).collect()
}

fn words_in(normalized: &str) -> usize {
    normalized.split(' ').filter(|w| !w.is_empty()).count()
}

/// Le numéro d'un identifiant de site de la personne (`recherche.12` → 12), au moins [`FIRST_CUSTOM`].
fn custom_number(id: &str) -> Option<u32> {
    id.strip_prefix(PREFIX)?.parse::<u32>().ok().filter(|n| *n >= FIRST_CUSTOM)
}

impl WebSearch {
    /// Le prochain identifiant libre pour un site de la personne.
    pub fn next_id(existing: &[WebSearch]) -> String {
        let n = existing.iter().filter_map(|w| custom_number(&w.id)).max().map_or(FIRST_CUSTOM, |m| m + 1);
        format!("{PREFIX}{n}")
    }

    /// Les sites lus dans un fichier de réglages écrit à la main : nom, modèle d'adresse **valable** et au moins une phrase exigés ;
    /// identifiants `recherche.<n>` (n ≥ 10) uniques ; un site mal écrit est écarté seul, les autres sont gardés. Une phrase de moins de
    /// deux mots, déjà prise par un site livré ou par un site précédent, ou trop longue est écartée.
    pub fn cleaned(list: Vec<WebSearch>) -> Vec<WebSearch> {
        let taken_builtin = builtin_phrases();
        let mut seen_phrases: Vec<String> = Vec::new();
        let mut out: Vec<WebSearch> = Vec::new();
        for mut site in list {
            if out.len() >= MAX_SITES {
                break;
            }
            site.name = site.name.split_whitespace().collect::<Vec<_>>().join(" ");
            site.url = site.url.trim().to_string();
            let mut mine: Vec<String> = Vec::new();
            site.phrases = site
                .phrases
                .into_iter()
                .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|p| {
                    let n = normalize(p);
                    let keep = words_in(&n) >= 2
                        && p.chars().count() <= MAX_PHRASE
                        && !taken_builtin.contains(&n)
                        && !seen_phrases.contains(&n)
                        && !mine.contains(&n);
                    if keep {
                        mine.push(n);
                    }
                    keep
                })
                .take(MAX_PHRASES)
                .collect();
            if custom_number(&site.id).is_some()
                && !site.name.is_empty()
                && site.name.chars().count() <= MAX_NAME
                && !site.phrases.is_empty()
                && check_template(&site.url).is_ok()
                && !out.iter().any(|o| o.id == site.id)
            {
                seen_phrases.extend(mine);
                out.push(site);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(id: &str, name: &str, url: &str, phrases: &[&str]) -> WebSearch {
        WebSearch { id: id.into(), name: name.into(), url: url.into(), phrases: phrases.iter().map(|p| (*p).to_string()).collect() }
    }

    /// Un modèle d'adresse n'est valable qu'en `https`, à hôte fixe, avec un seul `{q}` dans le chemin ou la requête (défaut B4 du cadrage).
    #[test]
    fn commands_511_a_template_must_be_https_with_a_fixed_host_and_one_placeholder() {
        for good in [
            "https://www.google.com/search?q={q}",
            "https://example.com/{q}",
            "HTTPS://Example.COM:8443/s?x=1&q={q}&y=2",
            "https://fr.wikipedia.org/wiki/Special:Search/{q}",
        ] {
            assert!(check_template(good).is_ok(), "{good}");
        }
        assert_eq!(check_template("https://www.google.com/search?q={q}").as_deref(), Ok("www.google.com"));
        assert_eq!(check_template("HTTPS://Example.COM:8443/{q}").as_deref(), Ok("example.com"), "hôte en minuscules, sans le port");
        for (bad, why) in [
            ("http://example.com/?q={q}", TemplateError::NotHttps),
            ("javascript:alert({q})", TemplateError::NotHttps),
            ("file:///etc/passwd?{q}", TemplateError::NotHttps),
            ("ftp://example.com/{q}", TemplateError::NotHttps),
            ("//example.com/{q}", TemplateError::NotHttps),
            ("example.com/{q}", TemplateError::NotHttps),
            ("", TemplateError::NotHttps),
            ("https://{q}", TemplateError::Host),
            ("https://{q}.example.com/", TemplateError::Host),
            ("https://example.com{q}", TemplateError::Host),
            ("https://example.com:{q}/", TemplateError::Host),
            ("https://user@example.com/?q={q}", TemplateError::Host),
            ("https://example.com@evil.com/?q={q}", TemplateError::Host),
            ("https://example.com%2f@evil.com/?q={q}", TemplateError::Host),
            ("https://localhost/?q={q}", TemplateError::Host),
            ("https://192.168.0.1/?q={q}", TemplateError::Host),
            ("https://[::1]/?q={q}", TemplateError::Host),
            ("https://-bad.example.com/?q={q}", TemplateError::Host),
            ("https://example..com/?q={q}", TemplateError::Host),
            ("https://example.com/#{q}", TemplateError::Host),
            ("https://example.com/?q=", TemplateError::Placeholder),
            ("https://example.com/?q={q}&r={q}", TemplateError::Placeholder),
            ("https://example.com/?q={Q}", TemplateError::Placeholder),
            ("https://example.com/?q={q}{", TemplateError::Placeholder),
            ("https://example.com/?q={q} x", TemplateError::BadCharacter),
            ("https://example.com/?q=\"{q}\"", TemplateError::BadCharacter),
            ("https://example.com/<{q}>", TemplateError::BadCharacter),
            ("https://example.com/\\{q}", TemplateError::BadCharacter),
            ("https://example.com/?q={q}\n", TemplateError::BadCharacter),
            ("https://exämple.com/?q={q}", TemplateError::BadCharacter),
        ] {
            assert_eq!(check_template(bad), Err(why), "{bad}");
        }
        let long = format!("https://example.com/{}{{q}}", "a".repeat(MAX_TEMPLATE));
        assert_eq!(check_template(&long), Err(TemplateError::NotHttps));
    }

    /// Le texte à encoder : tout ce qui n'est pas non réservé est encodé ; la requête ne peut rien ajouter à l'adresse (ni `/`, ni `@`, `#`, `&`).
    #[test]
    fn commands_512_the_query_is_encoded_and_cannot_change_the_address() {
        let g = "https://www.google.com/search?q={q}";
        assert_eq!(search_url(g, "chats mignons").as_deref(), Some("https://www.google.com/search?q=chats%20mignons"));
        assert_eq!(search_url(g, "été à Paris").as_deref(), Some("https://www.google.com/search?q=%C3%A9t%C3%A9%20%C3%A0%20Paris"));
        let hostile = "evil.com/x?y=1&z=2#frag@host \" < > \\ { } %41";
        let url = search_url(g, hostile).unwrap();
        assert_eq!(url.matches('?').count(), 1);
        assert_eq!(url.matches('#').count() + url.matches('@').count() + url.matches('&').count(), 0, "{url}");
        assert!(url.starts_with("https://www.google.com/search?q=evil.com%2Fx%3Fy%3D1%26z%3D2%23frag%40host%20"));
        assert!(url.contains("%2541"), "le « % » dit est encodé, pas interprété");
        // Un modèle sans hôte fixe ne donne rien, même si la requête ressemble à un hôte.
        assert_eq!(search_url("https://{q}", "evil.com"), None);
        assert_eq!(search_url("https://example.com{q}", "evil.com"), None);
        // Blancs et sauts de ligne ramenés à une espace ; rien à chercher : rien.
        assert_eq!(search_url(g, "a \n\t b\r\nc").as_deref(), Some("https://www.google.com/search?q=a%20b%20c"));
        assert_eq!(search_url(g, "   \n"), None);
        assert_eq!(search_url(g, ""), None);
        // Requête trop longue : coupée.
        let long = "é".repeat(MAX_QUERY_CHARS + 50);
        let url = search_url(g, &long).unwrap();
        assert_eq!(url.matches("%C3%A9").count(), MAX_QUERY_CHARS);
        assert!(url.len() <= MAX_URL);
    }

    /// Les quatre sites livrés, Wikipédia dans la langue de la phrase dite.
    #[test]
    fn commands_513_the_four_builtin_sites_and_the_wikipedia_language() {
        let settings = AppSettings::default();
        let t = |id: &str, lang, q: &str| target(id, lang, q, &settings);
        assert_eq!(t("recherche.1", Lang::Fr, "chats").as_deref(), Some("https://www.google.com/search?q=chats"));
        assert_eq!(t("recherche.2", Lang::Fr, "chats").as_deref(), Some("https://www.youtube.com/results?search_query=chats"));
        assert_eq!(t("recherche.3", Lang::Fr, "chats").as_deref(), Some("https://fr.wikipedia.org/w/index.php?search=chats"));
        assert_eq!(t("recherche.3", Lang::En, "cats").as_deref(), Some("https://en.wikipedia.org/w/index.php?search=cats"));
        assert_eq!(t("recherche.4", Lang::En, "cats").as_deref(), Some("https://duckduckgo.com/?q=cats"));
        assert_eq!(t("recherche.9", Lang::Fr, "chats"), None, "numéro inconnu");
        assert_eq!(t("recherche.10", Lang::Fr, "chats"), None, "site de la personne absent des réglages");
        for site in &SITES {
            assert!(check_template(site.fr).is_ok() && check_template(site.en).is_ok(), "{}", site.id);
        }
    }

    /// Les sites de la personne sont contrôlés **à la lecture** des réglages : un site mal écrit est écarté seul, les autres sont gardés.
    #[test]
    fn commands_514_custom_sites_are_checked_when_the_settings_are_read() {
        let cleaned = WebSearch::cleaned(vec![
            site(
                "recherche.10",
                "  Amazon  ",
                " https://www.amazon.fr/s?k={q} ",
                &["nouvelle recherche amazon", "  nouvelle  recherche   AMAZON "],
            ),
            site("recherche.11", "Piège", "https://{q}", &["nouvelle recherche piège"]),
            site("recherche.12", "Http", "http://example.com/?q={q}", &["nouvelle recherche http"]),
            site("recherche.13", "Sans phrase", "https://example.com/?q={q}", &[]),
            site("recherche.14", "Un mot", "https://example.com/?q={q}", &["amazon"]),
            site("recherche.15", "Google bis", "https://example.org/?q={q}", &["Nouvelle recherche Google"]),
            site("recherche.3", "Numéro livré", "https://example.org/?q={q}", &["nouvelle recherche ailleurs"]),
            site("perso.1", "Mauvais préfixe", "https://example.org/?q={q}", &["nouvelle recherche ailleurs"]),
            site("recherche.10", "Doublon", "https://example.org/?q={q}", &["nouvelle recherche doublon"]),
            site("recherche.16", "Phrase reprise", "https://example.org/?q={q}", &["nouvelle recherche amazon"]),
            site("recherche.17", "", "https://example.org/?q={q}", &["nouvelle recherche rien"]),
        ]);
        let ids: Vec<&str> = cleaned.iter().map(|w| w.id.as_str()).collect();
        assert_eq!(ids, ["recherche.10"], "{cleaned:?}");
        assert_eq!(cleaned[0].name, "Amazon");
        assert_eq!(cleaned[0].url, "https://www.amazon.fr/s?k={q}");
        assert_eq!(cleaned[0].phrases, vec!["nouvelle recherche amazon"], "espaces ramenés à un, doublon écarté");
        // Depuis un fichier de réglages : la lecture applique le même contrôle.
        let settings = AppSettings::from_json(
            r#"{"webSearches": [
                {"id": "recherche.10", "name": "Amazon", "url": "https://www.amazon.fr/s?k={q}", "phrases": ["nouvelle recherche amazon"]},
                {"id": "recherche.11", "name": "Piège", "url": "https://{q}", "phrases": ["nouvelle recherche piège"]},
                {"pas": "lisible"}
            ]}"#,
        )
        .unwrap();
        assert_eq!(settings.web_searches.len(), 1);
        assert_eq!(target("recherche.10", Lang::Fr, "stylo bleu", &settings).as_deref(), Some("https://www.amazon.fr/s?k=stylo%20bleu"));
        assert_eq!(AppSettings::from_json(r#"{"webSearches": 3}"#).unwrap().web_searches, vec![]);
        assert_eq!(WebSearch::next_id(&settings.web_searches), "recherche.11");
        assert_eq!(WebSearch::next_id(&[]), "recherche.10");
    }

    #[test]
    fn commands_515_only_digit_ids_are_searches() {
        assert!(is_search("recherche.1") && is_search("recherche.42"));
        assert!(!is_search("recherche.") && !is_search("recherche.x") && !is_search("perso.1") && !is_search("extrait.1"));
    }
}
