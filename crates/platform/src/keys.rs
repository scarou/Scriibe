//! Touche globale : le système annonce chaque changement d'état d'une touche (appuyée ou relâchée) avec toutes celles qui sont tenues.
//! La machine à états (appui, relâchement, traduction, annulation) est dans `scriibe_core::hotkey`.

use scriibe_core::hotkey::{Binding, Key};
use scriibe_core::settings::{AppSettings, Hotkey};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyEvent {
    /// Une touche a changé d'état ; `held` : toutes celles qui sont tenues maintenant. Les touches Ctrl et Maj ont leur nom, les autres
    /// (lettres, Alt, touches de fonction…) sont des `Key::Vk`.
    Changed { changed: Key, down: bool, held: Vec<Key> },
}

/// Un abonnement qui s'arrête quand on le lâche.
pub trait Subscription: Send {}

pub trait KeyListener: Send + Sync {
    /// Commence à envoyer les événements dans `sink`. Les touches ne sont jamais retenues : l'appli active les reçoit comme avant.
    /// Le rappel du système doit rester très court : on ne fait que poster l'événement.
    fn start(&self, sink: Sender<KeyEvent>) -> Result<Box<dyn Subscription>, String>;
}

/// Codes de touche virtuels de Windows (`VK_*`) des touches de modification que la machine à états connaît par leur nom.
pub mod vk {
    pub const LSHIFT: u32 = 0xA0;
    pub const RSHIFT: u32 = 0xA1;
    pub const LCONTROL: u32 = 0xA2;
    pub const RCONTROL: u32 = 0xA3;
    pub const LMENU: u32 = 0xA4;
}

pub const fn key_of(vk: u32) -> Key {
    match vk {
        vk::LSHIFT => Key::LeftShift,
        vk::RSHIFT => Key::RightShift,
        vk::LCONTROL => Key::LeftControl,
        vk::RCONTROL => Key::RightControl,
        other => Key::Vk(other),
    }
}

/// Transforme un événement de touche de Windows (`vk` : code de touche, `down` : enfoncée ou relâchée) en événement de l'appli.
/// `held` : les touches déjà tenues, mises à jour ici. Le maintien d'une touche enfoncée (répétition) ne change rien.
pub fn translate_vk(held: &mut Vec<Key>, vk: u32, down: bool) -> Option<KeyEvent> {
    translate_key(held, key_of(vk), down)
}

/// Comme `translate_vk` pour un système qui nomme lui-même ses touches (Option, Commande et Fn du Mac).
pub fn translate_key(held: &mut Vec<Key>, key: Key, down: bool) -> Option<KeyEvent> {
    let already = held.contains(&key);
    if down == already {
        return None;
    }
    if down {
        // Une touche dont le relâchement n'a jamais été vu (écran verrouillé…) ne reste pas indéfiniment.
        if held.len() >= MAX_HELD {
            held.remove(0);
        }
        held.push(key);
    } else {
        held.retain(|k| *k != key);
    }
    Some(KeyEvent::Changed { changed: key, down, held: held.clone() })
}

const MAX_HELD: usize = 24;

/// Une touche qu'on peut utiliser pour dicter.
pub struct KeyChoice {
    pub id: &'static str,
    pub label: &'static str,
    /// Code de touche virtuel de Windows (0 pour une touche propre au Mac : Option, Commande, Fn).
    pub vk: u32,
    /// La touche telle que la machine à états la connaît.
    pub key: Key,
    /// Touche de modification (Ctrl, Alt, Maj, Windows ; Option, Commande, Fn sur Mac) : se tient avec une autre ; sinon, une touche
    /// ordinaire qui ne sert pas à taper.
    pub modifier: bool,
}

const fn choice(id: &'static str, label: &'static str, vk: u32, modifier: bool) -> KeyChoice {
    KeyChoice { id, label, vk, key: key_of(vk), modifier }
}

/// Une touche de modification propre au Mac (nommée par la machine à états, sans code de Windows).
const fn named(id: &'static str, label: &'static str, key: Key) -> KeyChoice {
    KeyChoice { id, label, vk: 0, key, modifier: true }
}

pub const VK_LMENU: u32 = 0xA4;
pub const VK_LWIN: u32 = 0x5B;
pub const VK_RWIN: u32 = 0x5C;

/// Les touches qu'on peut utiliser (Windows). Ni lettres, chiffres, Espace, Entrée ou flèches (elles servent à taper), ni AltGr (Alt droit :
/// nécessaire à « @ » et « € » sur un clavier AZERTY).
pub const CHOICES: &[KeyChoice] = &[
    choice("right_control", "Ctrl droit", vk::RCONTROL, true),
    choice("left_control", "Ctrl gauche", vk::LCONTROL, true),
    choice("left_alt", "Alt gauche", VK_LMENU, true),
    choice("left_shift", "Maj gauche", vk::LSHIFT, true),
    choice("right_shift", "Maj droit", vk::RSHIFT, true),
    choice("left_win", "Windows gauche", VK_LWIN, true),
    choice("right_win", "Windows droite", VK_RWIN, true),
    choice("oem_102", "< > (à gauche de W)", 0xE2, false),
    choice("f1", "F1", 0x70, false),
    choice("f2", "F2", 0x71, false),
    choice("f3", "F3", 0x72, false),
    choice("f4", "F4", 0x73, false),
    choice("f5", "F5", 0x74, false),
    choice("f6", "F6", 0x75, false),
    choice("f7", "F7", 0x76, false),
    choice("f8", "F8", 0x77, false),
    choice("f9", "F9", 0x78, false),
    choice("f10", "F10", 0x79, false),
    choice("f11", "F11", 0x7A, false),
    choice("f12", "F12", 0x7B, false),
    choice("f13", "F13", 0x7C, false),
    choice("f14", "F14", 0x7D, false),
    choice("f15", "F15", 0x7E, false),
    choice("f16", "F16", 0x7F, false),
    choice("f17", "F17", 0x80, false),
    choice("f18", "F18", 0x81, false),
    choice("f19", "F19", 0x82, false),
    choice("f20", "F20", 0x83, false),
    choice("f21", "F21", 0x84, false),
    choice("f22", "F22", 0x85, false),
    choice("f23", "F23", 0x86, false),
    choice("f24", "F24", 0x87, false),
    choice("pause", "Pause", 0x13, false),
    choice("scroll_lock", "Verr. défil.", 0x91, false),
    choice("print_screen", "Impr. écran", 0x2C, false),
    choice("menu", "Menu contextuel", 0x5D, false),
    choice("insert", "Inser", 0x2D, false),
    choice("home", "Début", 0x24, false),
    choice("end", "Fin", 0x23, false),
    choice("page_up", "Page préc.", 0x21, false),
    choice("page_down", "Page suiv.", 0x22, false),
];

/// Les touches qu'on peut utiliser (Mac). Les touches de modification du Mac, gauche et droite distinguées, puis les touches de fonction
/// que le Mac connaît (F1 à F20) et quatre touches de déplacement. Ni lettres, chiffres, Espace, Entrée, flèches, Verr. maj, Échap, Tab
/// ou Suppr (elles servent à taper), ni les touches propres à Windows.
pub const MAC_CHOICES: &[KeyChoice] = &[
    named("right_option", "Option droite", Key::RightOption),
    named("left_option", "Option gauche", Key::LeftOption),
    named("right_command", "Commande droite (⌘)", Key::RightCommand),
    named("left_command", "Commande gauche (⌘)", Key::LeftCommand),
    named("fn", "Fn", Key::Fn),
    choice("right_control", "Ctrl droit", vk::RCONTROL, true),
    choice("left_control", "Ctrl gauche", vk::LCONTROL, true),
    choice("left_shift", "Maj gauche", vk::LSHIFT, true),
    choice("right_shift", "Maj droit", vk::RSHIFT, true),
    choice("f1", "F1", 0x70, false),
    choice("f2", "F2", 0x71, false),
    choice("f3", "F3", 0x72, false),
    choice("f4", "F4", 0x73, false),
    choice("f5", "F5", 0x74, false),
    choice("f6", "F6", 0x75, false),
    choice("f7", "F7", 0x76, false),
    choice("f8", "F8", 0x77, false),
    choice("f9", "F9", 0x78, false),
    choice("f10", "F10", 0x79, false),
    choice("f11", "F11", 0x7A, false),
    choice("f12", "F12", 0x7B, false),
    choice("f13", "F13", 0x7C, false),
    choice("f14", "F14", 0x7D, false),
    choice("f15", "F15", 0x7E, false),
    choice("f16", "F16", 0x7F, false),
    choice("f17", "F17", 0x80, false),
    choice("f18", "F18", 0x81, false),
    choice("f19", "F19", 0x82, false),
    choice("f20", "F20", 0x83, false),
    choice("home", "Début", 0x24, false),
    choice("end", "Fin", 0x23, false),
    choice("page_up", "Page préc.", 0x21, false),
    choice("page_down", "Page suiv.", 0x22, false),
];

/// La touche à tenir en plus pour traduire, quand elle n'est pas réglée : Maj, l'une ou l'autre.
pub const BOTH_SHIFTS: &str = "shift";

/// Le temps à tenir une combinaison de touches de modification seules avant que l'écoute commence.
pub const HOLD_MS: u32 = 300;

/// Ce qui change d'un système à l'autre dans le réglage des touches : la liste proposée, la touche de traduction par défaut, les touches
/// seules qui écoutent tout de suite.
pub struct Layout {
    choices: &'static [KeyChoice],
    mac: bool,
    /// Les touches de modification qui, seules, commencent à écouter tout de suite (les autres attendent `HOLD_MS`).
    immediate: &'static [&'static str],
}

/// Windows et Linux : Ctrl droit dicte, Maj traduit.
pub const WINDOWS_LAYOUT: Layout = Layout { choices: CHOICES, mac: false, immediate: &["right_control"] };

/// Mac : comme dans Scribe, Option droite dicte et ⌘ traduit ; Option, ⌘ droite et Fn écoutent tout de suite.
pub const MAC_LAYOUT: Layout =
    Layout { choices: MAC_CHOICES, mac: true, immediate: &["right_option", "left_option", "right_command", "fn"] };

/// La disposition du système sur lequel tourne le programme.
pub fn layout() -> &'static Layout {
    if cfg!(target_os = "macos") {
        &MAC_LAYOUT
    } else {
        &WINDOWS_LAYOUT
    }
}

impl Layout {
    /// Les touches proposées.
    pub fn choices(&self) -> &'static [KeyChoice] {
        self.choices
    }

    fn find(&self, id: &str) -> Option<&'static KeyChoice> {
        self.choices.iter().find(|c| c.id == id)
    }

    fn find_by_key(&self, key: Key) -> Option<&'static KeyChoice> {
        self.choices.iter().find(|c| c.key == key)
    }

    /// Cette touche de modification, seule, écoute-t-elle tout de suite ?
    pub fn is_immediate(&self, id: &str) -> bool {
        self.immediate.contains(&id)
    }

    /// Les identifiants des touches de modification qui écoutent tout de suite, pour la page.
    pub fn immediate_ids(&self) -> &'static [&'static str] {
        self.immediate
    }

    /// L'identifiant de la touche de dictée par défaut (celle des réglages sur Mac, Ctrl droit ailleurs).
    pub fn default_dictation_id(&self, settings: &AppSettings) -> &'static str {
        if !self.mac {
            return "right_control";
        }
        match settings.hotkey {
            Hotkey::RightOption => "right_option",
            Hotkey::LeftOption => "left_option",
            Hotkey::RightCommand => "right_command",
            Hotkey::Fn => "fn",
            Hotkey::RightControl => "right_control",
        }
    }

    /// L'identifiant d'une touche de la liste (`f9`, `left_win`, `right_option`…), si elle y est.
    pub fn id_of(&self, key: Key) -> Option<&'static str> {
        self.find_by_key(key).map(|c| c.id)
    }

    /// Le nom d'une touche appuyée pendant la capture : celui de la liste, sinon la lettre ou le code (« touche 0x41 »).
    pub fn label_of(&self, key: Key) -> String {
        if let Some(choice) = self.find_by_key(key) {
            return choice.label.to_string();
        }
        match vk_of(key) {
            Some(code @ (0x30..=0x39 | 0x41..=0x5A)) => format!("la touche {}", char::from(code as u8)),
            Some(0x20) => "Espace".to_string(),
            Some(0x0D) => "Entrée".to_string(),
            Some(0xA5) => "AltGr (Alt droit)".to_string(),
            Some(code) => format!("la touche de code {code:#04x}"),
            None => "une touche inconnue".to_string(),
        }
    }

    /// La touche d'une touche vue par la page (`KeyboardEvent.code`). Sur Mac, Alt est Option et Meta est Commande.
    pub fn key_of_web_code(&self, code: &str) -> Option<Key> {
        if self.mac {
            match code {
                "AltLeft" => return Some(Key::LeftOption),
                "AltRight" => return Some(Key::RightOption),
                "MetaLeft" => return Some(Key::LeftCommand),
                "MetaRight" => return Some(Key::RightCommand),
                _ => {}
            }
        }
        vk_of_web_code(code).map(key_of)
    }

    /// « Ctrl gauche + F9 » : les touches d'une liste d'identifiants, pour l'affichage.
    pub fn label(&self, ids: &[String]) -> String {
        ids.iter().map(|id| self.find(id).map_or(id.as_str(), |c| c.label)).collect::<Vec<_>>().join(" + ")
    }

    /// Vérifie une combinaison de dictée et la touche de traduction. Une combinaison de touches de modification seules (Maj gauche, Ctrl + Alt…)
    /// est permise : elle n'ouvre le micro qu'après `HOLD_MS` ms, pour qu'un raccourci clavier ne le déclenche pas.
    pub fn validate(&self, dictation: &[String], translate: &[String]) -> Result<(), String> {
        if dictation.is_empty() {
            return if translate.is_empty() { Ok(()) } else { self.validate_translate(&[], translate) };
        }
        if dictation.len() > 3 {
            return Err("trois touches au plus".into());
        }
        let mut chosen = Vec::new();
        for id in dictation {
            let choice = self.find(id).ok_or_else(|| format!("touche inconnue : {id}"))?;
            if chosen.iter().any(|c: &&KeyChoice| c.id == choice.id) {
                return Err(format!("{} choisie deux fois", choice.label));
            }
            chosen.push(choice);
        }
        if chosen.iter().filter(|c| !c.modifier).count() > 1 {
            return Err(if self.mac {
                "une seule touche autre que Ctrl, Maj, Option, ⌘ ou Fn dans la combinaison"
            } else {
                "une seule touche autre que Ctrl, Alt, Maj ou Windows dans la combinaison"
            }
            .into());
        }
        self.validate_translate(&chosen, translate)
    }

    fn validate_translate(&self, dictation: &[&KeyChoice], translate: &[String]) -> Result<(), String> {
        match translate {
            [] => {
                if self.mac {
                    // Par défaut ⌘ (l'une ou l'autre) traduit : il en faut au moins une qui ne serve pas à dicter.
                    if dictation.iter().any(|c| c.id == "left_command") && dictation.iter().any(|c| c.id == "right_command") {
                        return Err("la dictée utilise ⌘ : choisis une autre touche pour traduire".into());
                    }
                } else if dictation.iter().any(|c| c.id == "left_shift" || c.id == "right_shift") {
                    return Err("la dictée utilise Maj : choisis une autre touche pour traduire".into());
                }
                Ok(())
            }
            [id] if id == BOTH_SHIFTS => self.validate_translate(dictation, &[]),
            [id] => {
                let choice = self.find(id).ok_or_else(|| format!("touche inconnue : {id}"))?;
                if dictation.iter().any(|c| c.id == choice.id) {
                    return Err(format!("{} sert déjà à dicter", choice.label));
                }
                Ok(())
            }
            _ => Err("une seule touche pour traduire".into()),
        }
    }

    /// Les touches de dictée et de traduction d'une combinaison réglée (vérifiée par `validate`), sinon `None`.
    pub fn custom_binding(&self, dictation: &[String], translate: &[String]) -> Option<Binding> {
        if dictation.is_empty() || self.validate(dictation, translate).is_err() {
            return None;
        }
        let choices = dictation.iter().map(|id| self.find(id)).collect::<Option<Vec<_>>>()?;
        let keys: Vec<Key> = choices.iter().map(|c| c.key).collect();
        let default_translate = || {
            if self.mac {
                // ⌘, mais pas celle qui sert déjà à dicter (ce que fait `Binding::from_hotkey` pour ⌘ droite).
                [Key::LeftCommand, Key::RightCommand].into_iter().filter(|k| !keys.contains(k)).collect()
            } else {
                vec![Key::LeftShift, Key::RightShift]
            }
        };
        let translate = match translate {
            [] => default_translate(),
            [id] if id == BOTH_SHIFTS => default_translate(),
            [id] => vec![self.find(id)?.key],
            _ => return None,
        };
        // Les touches de modification qui écoutent tout de suite (Ctrl droit sous Windows et Linux ; Option, ⌘ droite et Fn sur Mac, comme
        // dans Scribe) le font seules ; les autres touches de modification seules attendent.
        let only_modifiers = choices.iter().all(|c| c.modifier);
        let hold_ms = if only_modifiers && !(choices.len() == 1 && self.is_immediate(choices[0].id)) { HOLD_MS } else { 0 };
        Some(Binding { dictation: keys, translate, hold_ms })
    }
}

fn find_by_vk(code: u32) -> Option<&'static KeyChoice> {
    CHOICES.iter().find(|c| c.vk == code)
}

/// Le code de touche de Windows d'une touche (rien pour les touches du Mac).
pub fn vk_of(key: Key) -> Option<u32> {
    match key {
        Key::LeftShift => Some(vk::LSHIFT),
        Key::RightShift => Some(vk::RSHIFT),
        Key::LeftControl => Some(vk::LCONTROL),
        Key::RightControl => Some(vk::RCONTROL),
        Key::Vk(code) => Some(code),
        _ => None,
    }
}

/// Le code de touche de Windows d'une touche vue par la page (`KeyboardEvent.code`) : la fenêtre de Scriibe ne reçoit pas les touches du
/// crochet global quand elle a le focus, la page les rapporte donc elle-même pendant une capture.
pub fn vk_of_web_code(code: &str) -> Option<u32> {
    let fixed = match code {
        "ControlLeft" => vk::LCONTROL,
        "ControlRight" => vk::RCONTROL,
        "ShiftLeft" => vk::LSHIFT,
        "ShiftRight" => vk::RSHIFT,
        "AltLeft" => VK_LMENU,
        "AltRight" => 0xA5,
        "MetaLeft" => VK_LWIN,
        "MetaRight" => VK_RWIN,
        "IntlBackslash" => 0xE2,
        "Pause" => 0x13,
        "ScrollLock" => 0x91,
        "PrintScreen" => 0x2C,
        "ContextMenu" => 0x5D,
        "Insert" => 0x2D,
        "Home" => 0x24,
        "End" => 0x23,
        "PageUp" => 0x21,
        "PageDown" => 0x22,
        "Space" => 0x20,
        "Enter" | "NumpadEnter" => 0x0D,
        "Tab" => 0x09,
        "Escape" => 0x1B,
        "Backspace" => 0x08,
        "Delete" => 0x2E,
        "CapsLock" => 0x14,
        "ArrowLeft" => 0x25,
        "ArrowUp" => 0x26,
        "ArrowRight" => 0x27,
        "ArrowDown" => 0x28,
        _ => 0,
    };
    if fixed != 0 {
        return Some(fixed);
    }
    if let Some(n) = code.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()).filter(|n| (1..=24).contains(n)) {
        return Some(0x6F + n);
    }
    if let Some(letter) = code.strip_prefix("Key").filter(|l| l.len() == 1) {
        return letter.bytes().next().filter(u8::is_ascii_uppercase).map(u32::from);
    }
    code.strip_prefix("Digit").filter(|d| d.len() == 1).and_then(|d| d.bytes().next()).filter(u8::is_ascii_digit).map(u32::from)
}

/// L'identifiant d'une touche de la liste du système (`f9`, `left_win`, `right_option`…), si elle y est.
pub fn id_of(key: Key) -> Option<&'static str> {
    layout().id_of(key)
}

/// Vérifie une combinaison de dictée et la touche de traduction (voir `Layout::validate`).
pub fn validate(dictation: &[String], translate: &[String]) -> Result<(), String> {
    layout().validate(dictation, translate)
}

/// Les touches de dictée et de traduction d'une combinaison réglée (vérifiée par `validate`), sinon `None`.
pub fn custom_binding(dictation: &[String], translate: &[String]) -> Option<Binding> {
    layout().custom_binding(dictation, translate)
}

/// La touche d'une touche vue par la page (`KeyboardEvent.code`).
pub fn key_of_web_code(code: &str) -> Option<Key> {
    layout().key_of_web_code(code)
}

/// Les touches en vigueur : celles des réglages (Windows et Mac) si elles sont valides, sinon celles par défaut du système (la touche de
/// traduction réglée seule, avec la dictée par défaut, compte aussi).
pub fn binding(settings: &AppSettings) -> Binding {
    let default = Binding::from_hotkey(settings.hotkey);
    if !cfg!(any(windows, target_os = "macos")) || (settings.dictation_keys.is_empty() && settings.translate_keys.is_empty()) {
        return default;
    }
    let dictation = if settings.dictation_keys.is_empty() {
        vec![layout().default_dictation_id(settings).to_string()]
    } else {
        settings.dictation_keys.clone()
    };
    custom_binding(&dictation, &settings.translate_keys).unwrap_or(default)
}

/// Ce que le crochet du clavier doit faire en plus de signaler les touches (calculé depuis les touches en vigueur).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HookRules {
    /// Alt gauche ou Windows font partie des touches : un appui sans autre touche ouvrirait le menu de l'appli ou le menu Démarrer, et
    /// ferait perdre le focus au collage. Le crochet glisse alors une touche sans effet pendant l'appui, ce qui annule cette ouverture.
    pub mask_menu: bool,
    /// Les touches ordinaires de la combinaison (F9, « < > »…) : le crochet les retient quand le reste de la combinaison est tenu, pour
    /// qu'elles n'écrivent rien dans l'appli.
    pub swallow: Vec<u32>,
    /// Les autres touches de la combinaison de dictée, tenues avant de retenir une touche ordinaire.
    pub needs: Vec<u32>,
}

pub fn hook_rules(binding: &Binding) -> HookRules {
    let codes: Vec<u32> = binding.dictation.iter().filter_map(|k| vk_of(*k)).collect();
    let translate: Vec<u32> = binding.translate.iter().filter_map(|k| vk_of(*k)).collect();
    let is_menu_key = |code: &u32| [VK_LMENU, VK_LWIN, VK_RWIN].contains(code);
    let is_plain = |code: &u32| find_by_vk(*code).is_some_and(|c| !c.modifier);
    let plain: Vec<u32> = codes.iter().copied().filter(is_plain).collect();
    HookRules {
        mask_menu: codes.iter().chain(translate.iter()).any(is_menu_key),
        needs: if plain.is_empty() { Vec::new() } else { codes.iter().copied().filter(|c| !plain.contains(c)).collect() },
        swallow: plain,
    }
}

static RULES: std::sync::Mutex<Option<HookRules>> = std::sync::Mutex::new(None);

/// Les règles que le crochet applique (changées quand les touches changent).
pub fn set_hook_rules(rules: HookRules) {
    if let Ok(mut slot) = RULES.lock() {
        *slot = Some(rules);
    }
}

pub fn current_hook_rules() -> HookRules {
    RULES.lock().ok().and_then(|slot| slot.clone()).unwrap_or_default()
}

/// Le nom d'une touche appuyée pendant la capture : celui de la liste, sinon la lettre ou le code (« touche 0x41 »).
pub fn label_of(key: Key) -> String {
    layout().label_of(key)
}

/// Le nom d'une touche ou d'une combinaison (« Ctrl gauche + F9 ») en anglais, pour le menu de l'icône ; la page a sa propre liste
/// (`ui/src/lib/locales/en-server.ts`). Un nom inconnu reste tel quel.
pub fn english_label(french: &str) -> String {
    french
        .split(" + ")
        .map(|part| match part {
            "Ctrl droit" => "Right Ctrl",
            "Option droite" => "Right Option",
            "Option gauche" => "Left Option",
            "Commande droite (⌘)" => "Right Command (⌘)",
            "Commande gauche (⌘)" => "Left Command (⌘)",
            "Ctrl gauche" => "Left Ctrl",
            "Alt gauche" => "Left Alt",
            "Maj gauche" => "Left Shift",
            "Maj droit" => "Right Shift",
            "Maj" => "Shift",
            "Windows gauche" => "Left Windows",
            "Windows droite" => "Right Windows",
            "< > (à gauche de W)" => "< > (left of W)",
            "Verr. défil." => "Scroll Lock",
            "Impr. écran" => "Print Screen",
            "Menu contextuel" => "Context menu",
            "Inser" => "Insert",
            "Début" => "Home",
            "Fin" => "End",
            "Page préc." => "Page Up",
            "Page suiv." => "Page Down",
            other => other,
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

/// La touche de dictée par défaut du système, pour l'affichage tant qu'aucune touche n'est choisie.
pub fn default_dictation_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "Option droite"
    } else {
        "Ctrl droit"
    }
}

/// La touche de traduction par défaut du système (Commande, à gauche ou à droite, sur Mac), pour l'affichage.
pub fn default_translate_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "⌘"
    } else {
        "Maj"
    }
}

/// « Ctrl gauche + F9 » : les touches d'une liste d'identifiants, pour l'affichage.
pub fn label(ids: &[String]) -> String {
    layout().label(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriibe_core::hotkey::{HotkeyEvent, HotkeyMonitor};

    // Les tests existants décrivent la disposition de Windows et de Linux, quel que soit le système qui les exécute ; ceux du Mac ont leurs
    // propres fonctions (`mac_*`).
    fn validate(dictation: &[String], translate: &[String]) -> Result<(), String> {
        WINDOWS_LAYOUT.validate(dictation, translate)
    }

    fn custom_binding(dictation: &[String], translate: &[String]) -> Option<Binding> {
        WINDOWS_LAYOUT.custom_binding(dictation, translate)
    }

    fn id_of(key: Key) -> Option<&'static str> {
        WINDOWS_LAYOUT.id_of(key)
    }

    /// Rejoue des événements Windows dans la machine à états du cœur, comme le fait le chef d'orchestre.
    fn run_with(binding: Binding, events: &[(u32, bool)]) -> String {
        let mut held = Vec::new();
        let mut monitor = HotkeyMonitor::with_binding(binding);
        let mut out = String::new();
        for (code, down) in events {
            if let Some(KeyEvent::Changed { changed, down, held }) = translate_vk(&mut held, *code, *down) {
                out.extend(monitor.handle_change(changed, down, &held).into_iter().map(HotkeyEvent::letter));
            }
        }
        out
    }

    fn run(events: &[(u32, bool)]) -> String {
        run_with(Binding::from_hotkey(Hotkey::RightControl), events)
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    const A: u32 = 0x41;
    const LALT: u32 = 0xA4;
    const F9: u32 = 0x78;

    #[test]
    fn web_codes_become_windows_codes() {
        assert_eq!(vk_of_web_code("F9"), Some(0x78));
        assert_eq!(vk_of_web_code("F24"), Some(0x87));
        assert_eq!(vk_of_web_code("F25"), None);
        assert_eq!(vk_of_web_code("ControlLeft"), Some(vk::LCONTROL));
        assert_eq!(vk_of_web_code("IntlBackslash"), Some(0xE2));
        assert_eq!(vk_of_web_code("KeyV"), Some(0x56));
        assert_eq!(vk_of_web_code("Digit5"), Some(0x35));
        assert_eq!(vk_of_web_code("Unidentified"), None);
        // Les touches de la liste des choix gardent leur code.
        assert_eq!(id_of(key_of(vk_of_web_code("Pause").unwrap())), Some("pause"));
    }

    #[test]
    fn right_control_alone_is_a_dictation() {
        assert_eq!(run(&[(vk::RCONTROL, true), (vk::RCONTROL, false)]), "PR");
    }

    #[test]
    fn key_repeat_does_not_press_twice() {
        assert_eq!(run(&[(vk::RCONTROL, true), (vk::RCONTROL, true), (vk::RCONTROL, true), (vk::RCONTROL, false)]), "PR");
    }

    #[test]
    fn shift_added_translates() {
        assert_eq!(run(&[(vk::RCONTROL, true), (vk::LSHIFT, true), (vk::LSHIFT, false), (vk::RCONTROL, false)]), "PCR");
    }

    #[test]
    fn a_shortcut_cancels() {
        // Ctrl droit + A : un raccourci, pas une dictée.
        assert_eq!(run(&[(vk::RCONTROL, true), (A, true), (A, false), (vk::RCONTROL, false)]), "PXR");
        // Ctrl droit + Alt (hors AltGr) aussi.
        assert_eq!(run(&[(vk::RCONTROL, true), (LALT, true), (LALT, false), (vk::RCONTROL, false)]), "PXR");
    }

    #[test]
    fn typing_without_the_hotkey_is_ignored() {
        assert_eq!(run(&[(A, true), (A, false), (vk::LCONTROL, true), (A, true), (A, false), (vk::LCONTROL, false)]), "");
    }

    #[test]
    fn altgr_is_not_right_control() {
        // Sous Windows, AltGr arrive comme Ctrl gauche puis Alt droite : jamais Ctrl droit.
        const RALT: u32 = 0xA5;
        assert_eq!(run(&[(vk::LCONTROL, true), (RALT, true), (RALT, false), (vk::LCONTROL, false)]), "");
    }

    #[test]
    fn a_chosen_combination_dictates_and_a_chosen_key_translates() {
        let binding = custom_binding(&ids(&["left_control", "f9"]), &ids(&["left_alt"])).unwrap();
        assert_eq!(run_with(binding.clone(), &[(vk::LCONTROL, true), (F9, true), (F9, false), (vk::LCONTROL, false)]), "PR");
        // Alt gauche en plus : traduction (et non plus un raccourci qui annule).
        assert_eq!(run_with(binding.clone(), &[(vk::LCONTROL, true), (F9, true), (LALT, true), (F9, false)]), "PCR");
        // Ctrl gauche + C reste un raccourci ordinaire.
        assert_eq!(run_with(binding, &[(vk::LCONTROL, true), (0x43, true), (0x43, false), (vk::LCONTROL, false)]), "");
    }

    #[test]
    fn combinations_are_checked() {
        assert!(validate(&[], &[]).is_ok());
        assert!(validate(&ids(&["right_control"]), &[]).is_ok());
        assert!(validate(&ids(&["f9"]), &[]).is_ok());
        assert!(validate(&ids(&["left_control", "f9"]), &ids(&["left_shift"])).is_ok());
        assert!(validate(&ids(&["left_control", "left_alt"]), &[]).is_ok());
        // Des touches de modification seules sont permises : l'écoute n'y commence qu'après une courte attente.
        for id in ["left_control", "left_alt", "left_shift", "left_win", "right_win"] {
            assert!(validate(&ids(&[id]), &[]).is_ok() || id == "left_shift", "{id}");
        }
        assert!(validate(&ids(&["left_shift"]), &[]).unwrap_err().contains("Maj"), "Maj dicte : la traduction ne peut plus être Maj");
        assert!(validate(&ids(&["left_shift"]), &ids(&["left_alt"])).is_ok());
        assert!(validate(&ids(&["f9", "f10"]), &[]).unwrap_err().contains("une seule touche"));
        assert!(validate(&ids(&["a"]), &[]).unwrap_err().contains("inconnue"));
        assert!(validate(&ids(&["left_control", "left_control"]), &[]).unwrap_err().contains("deux fois"));
        assert!(validate(&ids(&["left_control", "f1", "left_alt", "f2"]), &[]).is_err());
        // La traduction ne réutilise pas une touche de dictée, ni Maj si Maj dicte.
        assert!(validate(&ids(&["f9"]), &ids(&["f9"])).unwrap_err().contains("dicter"));
        assert!(validate(&ids(&["left_shift", "f9"]), &[]).unwrap_err().contains("Maj"));
        assert!(validate(&ids(&["f9"]), &ids(&["f1", "f2"])).is_err());
        // Réglée seule, la traduction est vérifiée contre la dictée par défaut (Ctrl droit).
        assert!(validate(&[], &ids(&["left_alt"])).is_ok());
    }

    #[test]
    fn the_effective_binding_falls_back_to_the_default_when_the_settings_are_invalid() {
        let mut settings = AppSettings { hotkey: Hotkey::RightControl, ..AppSettings::default() };
        assert_eq!(binding(&settings), Binding::from_hotkey(Hotkey::RightControl));
        settings.dictation_keys = ids(&["nimporte-quoi"]);
        assert_eq!(binding(&settings), Binding::from_hotkey(Hotkey::RightControl));
        settings.dictation_keys = ids(&["left_control", "f9"]);
        let effective = binding(&settings);
        // Sur Windows la combinaison s'applique ; ailleurs, la touche par défaut du système.
        assert_eq!(effective == Binding::from_hotkey(Hotkey::RightControl), !cfg!(any(windows, target_os = "macos")));
    }

    #[test]
    fn modifier_only_combinations_wait_before_listening_but_right_control_and_ordinary_keys_do_not() {
        let hold = |list: &[&str]| custom_binding(&ids(list), &ids(&["left_alt"])).unwrap().hold_ms;
        assert_eq!(custom_binding(&ids(&["right_control"]), &[]).unwrap().hold_ms, 0);
        assert_eq!(hold(&["left_control"]), HOLD_MS);
        assert_eq!(hold(&["left_shift", "left_control"]), HOLD_MS);
        assert_eq!(hold(&["left_win"]), HOLD_MS);
        assert_eq!(hold(&["f9"]), 0);
        assert_eq!(hold(&["oem_102"]), 0);
        assert_eq!(hold(&["left_control", "f9"]), 0);
    }

    #[test]
    fn the_hook_masks_the_menu_keys_and_holds_back_ordinary_keys_of_the_combination() {
        let rules = |dictation: &[&str], translate: &[&str]| hook_rules(&custom_binding(&ids(dictation), &ids(translate)).unwrap());
        assert_eq!(rules(&["right_control"], &[]), HookRules::default());
        assert!(rules(&["left_win"], &[]).mask_menu);
        assert!(rules(&["left_control", "f9"], &["left_alt"]).mask_menu, "Alt gauche pour traduire compte aussi");
        let combo = rules(&["left_control", "f9"], &[]);
        assert_eq!((combo.swallow, combo.needs), (vec![0x78], vec![vk::LCONTROL]));
        let alone = rules(&["oem_102"], &[]);
        assert_eq!((alone.swallow, alone.needs.len()), (vec![0xE2], 0), "touche seule : retenue sans condition");
        assert!(rules(&["left_shift", "left_control"], &["left_alt"]).swallow.is_empty(), "des modificateurs ne sont jamais retenus");
    }

    #[test]
    fn a_pressed_key_is_named_from_the_list_or_not_at_all() {
        assert_eq!(id_of(Key::Vk(0x78)), Some("f9"));
        assert_eq!(id_of(Key::LeftControl), Some("left_control"));
        assert_eq!(id_of(Key::Vk(0x41)), None, "une lettre ne peut pas dicter");
        assert_eq!(id_of(Key::Vk(0xA5)), None, "AltGr non plus");
        assert_eq!(id_of(Key::Fn), None);
    }

    #[test]
    fn labels_have_an_english_form() {
        assert_eq!(english_label("Ctrl gauche + F9"), "Left Ctrl + F9");
        assert_eq!(english_label("Ctrl droit"), "Right Ctrl");
        assert_eq!(english_label("Maj"), "Shift");
        assert_eq!(english_label("Truc"), "Truc");
        // Chaque nom de la liste a sa forme anglaise (ou n'a pas de mot français : F1…F24, Pause).
        for choice in CHOICES.iter().chain(MAC_CHOICES) {
            let english = english_label(choice.label);
            assert!(
                english != choice.label || choice.label.chars().all(|c| c.is_ascii_alphanumeric()),
                "{} sans forme anglaise",
                choice.label
            );
        }
    }

    #[test]
    fn labels_are_readable() {
        assert_eq!(label(&ids(&["left_control", "f9"])), "Ctrl gauche + F9");
    }

    #[test]
    fn every_choice_has_a_unique_id_and_code() {
        for (i, a) in CHOICES.iter().enumerate() {
            for b in CHOICES.iter().skip(i + 1) {
                assert!(a.id != b.id && a.vk != b.vk, "{} / {}", a.id, b.id);
            }
        }
    }
    // ---- Mac (S13) : Option, Commande et Fn réglables ----

    fn mac_binding(dictation: &[&str], translate: &[&str]) -> Binding {
        MAC_LAYOUT.custom_binding(&ids(dictation), &ids(translate)).unwrap()
    }

    #[test]
    fn mac_the_list_has_the_modifier_keys_of_the_mac_the_function_keys_and_no_typing_key() {
        let list: Vec<&str> = MAC_CHOICES.iter().map(|c| c.id).collect();
        for id in [
            "right_option",
            "left_option",
            "right_command",
            "left_command",
            "fn",
            "right_control",
            "left_control",
            "left_shift",
            "right_shift",
        ] {
            assert!(MAC_LAYOUT.find(id).is_some_and(|c| c.modifier), "{id}");
        }
        for n in 1..=20 {
            assert!(list.contains(&format!("f{n}").as_str()), "F{n}");
        }
        for id in ["home", "end", "page_up", "page_down"] {
            assert!(MAC_LAYOUT.find(id).is_some_and(|c| !c.modifier), "{id}");
        }
        for absent in ["left_alt", "left_win", "right_win", "pause", "print_screen", "scroll_lock", "menu", "insert", "oem_102", "f21"] {
            assert!(!list.contains(&absent), "{absent} n'existe pas sur le Mac");
        }
        // Ni identifiant ni touche en double.
        for (i, a) in MAC_CHOICES.iter().enumerate() {
            for b in MAC_CHOICES.iter().skip(i + 1) {
                assert_ne!(a.id, b.id);
                assert_ne!(a.key, b.key, "{} et {}", a.id, b.id);
            }
        }
    }

    #[test]
    fn mac_default_keys_are_the_ones_of_scribe() {
        // Option droite seule dicte tout de suite, ⌘ (l'une ou l'autre) traduit : comme `Binding::from_hotkey`.
        let settings = AppSettings::default();
        assert_eq!(MAC_LAYOUT.default_dictation_id(&settings), "right_option");
        let explicit = mac_binding(&["right_option"], &[]);
        assert_eq!(explicit, Binding::from_hotkey(Hotkey::RightOption));
        let fn_key = mac_binding(&["fn"], &[]);
        assert_eq!((fn_key.dictation.clone(), fn_key.hold_ms), (vec![Key::Fn], 0));
        // ⌘ droite dicte : seule la ⌘ gauche traduit, comme dans Scribe.
        assert_eq!(mac_binding(&["right_command"], &[]), Binding::from_hotkey(Hotkey::RightCommand));
        // La touche par défaut suit le réglage de Scribe.
        let left = AppSettings { hotkey: Hotkey::LeftOption, ..AppSettings::default() };
        assert_eq!(MAC_LAYOUT.default_dictation_id(&left), "left_option");
        assert_eq!(WINDOWS_LAYOUT.default_dictation_id(&left), "right_control");
    }

    #[test]
    fn mac_modifier_keys_alone_wait_unless_they_are_the_immediate_ones() {
        assert_eq!(mac_binding(&["right_option"], &[]).hold_ms, 0);
        assert_eq!(mac_binding(&["left_option"], &[]).hold_ms, 0);
        assert_eq!(mac_binding(&["right_command"], &[]).hold_ms, 0);
        assert_eq!(mac_binding(&["left_command"], &[]).hold_ms, HOLD_MS, "⌘ gauche sert aux raccourcis");
        assert_eq!(mac_binding(&["right_control"], &[]).hold_ms, HOLD_MS, "Ctrl droit n'est immédiat que sous Windows");
        assert_eq!(mac_binding(&["left_shift"], &[]).hold_ms, HOLD_MS);
        assert_eq!(mac_binding(&["right_option", "left_control"], &[]).hold_ms, HOLD_MS);
        assert_eq!(mac_binding(&["f9"], &[]).hold_ms, 0);
        assert_eq!(mac_binding(&["left_control", "f9"], &[]).hold_ms, 0);
    }

    #[test]
    fn mac_combinations_are_checked() {
        let check = |d: &[&str], t: &[&str]| MAC_LAYOUT.validate(&ids(d), &ids(t));
        assert!(check(&[], &[]).is_ok());
        assert!(check(&["right_option"], &[]).is_ok());
        assert!(check(&["left_shift"], &[]).is_ok(), "Maj ne traduit pas sur Mac : il peut dicter");
        assert!(check(&["right_option", "f9"], &["left_shift"]).is_ok());
        assert!(check(&["left_win"], &[]).unwrap_err().contains("inconnue"));
        assert!(check(&["left_alt"], &[]).unwrap_err().contains("inconnue"));
        assert!(check(&["f9", "f10"], &[]).unwrap_err().contains("Option"), "le message cite les touches du Mac");
        assert!(check(&["right_option"], &["right_option"]).unwrap_err().contains("dicter"));
        assert!(check(&["left_command", "right_command"], &[]).unwrap_err().contains("⌘"));
        assert!(check(&["left_command", "right_command"], &["fn"]).is_ok());
        assert!(check(&["right_option"], &["f1", "f2"]).is_err());
        // Les messages sont ceux que l'interface traduit.
        assert_eq!(check(&["a"], &[]).unwrap_err(), "touche inconnue : a");
    }

    #[test]
    fn mac_the_translate_key_defaults_to_the_command_not_used_for_dictation() {
        let translate = |d: &[&str], t: &[&str]| mac_binding(d, t).translate;
        assert_eq!(translate(&["right_option"], &[]), vec![Key::LeftCommand, Key::RightCommand]);
        assert_eq!(translate(&["left_command", "f9"], &[]), vec![Key::RightCommand]);
        assert_eq!(translate(&["right_option"], &["left_shift"]), vec![Key::LeftShift]);
        assert_eq!(translate(&["right_option"], &[BOTH_SHIFTS]), vec![Key::LeftCommand, Key::RightCommand]);
    }

    /// Rejoue des événements nommés (touches du Mac) dans la machine à états.
    fn run_keys(binding: Binding, events: &[(Key, bool)]) -> String {
        let mut held = Vec::new();
        let mut monitor = HotkeyMonitor::with_binding(binding);
        let mut out = String::new();
        for (key, down) in events {
            let Some(KeyEvent::Changed { changed, down, held }) = translate_key(&mut held, *key, *down) else { continue };
            out.extend(monitor.handle_change(changed, down, &held).into_iter().map(HotkeyEvent::letter));
        }
        out
    }

    #[test]
    fn mac_a_chosen_combination_dictates_and_command_translates() {
        let binding = mac_binding(&["left_option", "f9"], &[]);
        let f9 = Key::Vk(F9);
        assert_eq!(run_keys(binding.clone(), &[(Key::LeftOption, true), (f9, true), (f9, false), (Key::LeftOption, false)]), "PR");
        assert_eq!(run_keys(binding.clone(), &[(Key::LeftOption, true), (f9, true), (Key::RightCommand, true), (f9, false)]), "PCR");
        // Option gauche seule, sans F9 : rien.
        assert_eq!(run_keys(binding, &[(Key::LeftOption, true), (Key::LeftOption, false)]), "");
    }

    #[test]
    fn mac_keys_are_named_from_the_list_and_the_page_uses_option_and_command() {
        assert_eq!(MAC_LAYOUT.id_of(Key::RightOption), Some("right_option"));
        assert_eq!(MAC_LAYOUT.id_of(Key::Fn), Some("fn"));
        assert_eq!(MAC_LAYOUT.id_of(Key::Vk(0x78)), Some("f9"));
        assert_eq!(MAC_LAYOUT.id_of(Key::Vk(0x41)), None, "une lettre ne peut pas dicter");
        assert_eq!(MAC_LAYOUT.id_of(Key::Vk(0x5B)), None, "la touche Windows n'existe pas sur Mac");
        assert_eq!(MAC_LAYOUT.label_of(Key::RightOption), "Option droite");
        assert_eq!(MAC_LAYOUT.label_of(Key::LeftCommand), "Commande gauche (⌘)");
        assert_eq!(MAC_LAYOUT.label_of(Key::Vk(0x41)), "la touche A");
        assert_eq!(MAC_LAYOUT.label(&ids(&["right_option", "f9"])), "Option droite + F9");
        // La page voit Alt et Meta : sur Mac, ce sont Option et Commande.
        assert_eq!(MAC_LAYOUT.key_of_web_code("AltRight"), Some(Key::RightOption));
        assert_eq!(MAC_LAYOUT.key_of_web_code("AltLeft"), Some(Key::LeftOption));
        assert_eq!(MAC_LAYOUT.key_of_web_code("MetaLeft"), Some(Key::LeftCommand));
        assert_eq!(MAC_LAYOUT.key_of_web_code("MetaRight"), Some(Key::RightCommand));
        assert_eq!(MAC_LAYOUT.key_of_web_code("ControlRight"), Some(Key::RightControl));
        assert_eq!(MAC_LAYOUT.key_of_web_code("F9"), Some(Key::Vk(0x78)));
        assert_eq!(MAC_LAYOUT.key_of_web_code("Unidentified"), None);
        assert_eq!(WINDOWS_LAYOUT.key_of_web_code("AltLeft"), Some(Key::Vk(VK_LMENU)));
        assert_eq!(WINDOWS_LAYOUT.key_of_web_code("MetaLeft"), Some(Key::Vk(VK_LWIN)));
    }

    #[test]
    fn mac_the_effective_binding_uses_the_chosen_keys() {
        let mut settings = AppSettings::default();
        assert_eq!(binding(&settings), Binding::from_hotkey(Hotkey::RightOption));
        settings.dictation_keys = ids(&["left_option", "f9"]);
        let effective = binding(&settings);
        assert_eq!(effective == Binding::from_hotkey(Hotkey::RightOption), !cfg!(any(windows, target_os = "macos")));
        // La touche de traduction réglée seule garde la dictée par défaut.
        let translate_only = AppSettings { translate_keys: ids(&["left_shift"]), ..AppSettings::default() };
        if cfg!(target_os = "macos") {
            let b = binding(&translate_only);
            assert_eq!((b.dictation, b.translate), (vec![Key::RightOption], vec![Key::LeftShift]));
        }
    }
}
