import type { CaptureColor } from './appearance';

/** Le client du serveur local : l'adresse de la page contient le jeton secret, les chemins sont donc relatifs. */

export interface ApiError {
  ok: false;
  error: string;
}

/** Marque posée avant le rechargement d'une page dont l'utilisateur n'était plus le bon : la page rechargée prévient que rien n'a été enregistré. */
export const STALE_FLAG = 'scriibe-stale-profile';

/**
 * L'utilisateur actif tel que la page le connaît : `undefined` tant que le serveur ne l'a pas dit, `null` s'il n'y en a pas. Il part avec chaque
 * requête qui écrit (clé `profile`) : si ce n'est plus l'utilisateur actif (changé depuis le menu de l'icône ou un autre onglet), le serveur
 * refuse en 409 sans rien écrire et la page se recharge, au lieu d'écraser les données d'un autre utilisateur.
 */
let knownProfile: string | null | undefined;

export function setKnownProfile(id: string | null | undefined): void {
  knownProfile = id;
}

export function getKnownProfile(): string | null | undefined {
  return knownProfile;
}

/** Le corps d'une requête avec l'utilisateur que la page croit actif (les corps qui ne sont pas des objets, ou qui le disent déjà, restent tels quels). */
export function withProfile(body: unknown, profile: string | null | undefined): unknown {
  if (profile === undefined || typeof body !== 'object' || body === null || Array.isArray(body) || 'profile' in body) return body;
  return { ...body, profile };
}

/** `keepalive` : la requête part même si la page se ferme ou se recharge pendant l'envoi (corps de 64 Ko au plus). */
export async function api<T = any>(path: string, body?: unknown, options: { keepalive?: boolean } = {}): Promise<T> {
  const payload = withProfile(body, knownProfile);
  const init: RequestInit =
    payload === undefined ? {} : { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(payload) };
  if (options.keepalive) init.keepalive = true;
  const response = await fetch('api/' + path, init);
  const out = await response.json();
  if (response.status === 409 && out?.stale) {
    try {
      sessionStorage.setItem(STALE_FLAG, '1');
    } catch {
      /* le rechargement suffit, seul le message est perdu */
    }
    location.reload();
  }
  return out;
}

export interface Status {
  version: string;
  windows: boolean;
  /** « windows », « macos » ou « linux » : choisit la couleur de la marque. */
  os: string;
  /** Le prénom du compte, pour le bandeau d'accueil (vide si inconnu). */
  first_name: string;
  /** Le système demande de réduire les animations, lu par Scriibe (Linux : WebKitGTK ne le transmet pas à la page). */
  reduced_motion?: boolean;
  recording: boolean;
  busy: boolean;
  keys_error: string | null;
  /** L'utilisateur actif (`null` tant que rien n'est rangé par utilisateur) et le nombre d'utilisateurs. */
  profile: { id: string; name: string } | null;
  profile_count: number;
  /** Une commande vocale (« ajouter au dictionnaire ») attend que la page prenne son texte. */
  prefill?: boolean;
  /** Sous Wayland (GNOME) : le raccourci de dictée choisi dans la fenêtre de GNOME (« Ctrl + ← ») ; `null` ailleurs. */
  gnome_shortcut: string | null;
  gnome_translate_shortcut: string | null;
  gnome_command_shortcut: string | null;
  /** Une version plus récente que celle qui tourne, trouvée par la vérification automatique (absent : à jour ou pas vérifié). */
  update: { sha: string; title: string; date: string } | null;
  /** Fichiers de données (`settings.json`, `dictionary.json`) que Scriibe refuse de réécrire : illisible, ou d'une version plus récente. */
  config_problems?: { file: string; reason: 'unreadable' | 'newer'; version: number | null; backup: string | null }[];
  hotkey: string;
  /** « Ctrl gauche + F9 » : la dictée et la traduction en vigueur. */
  dictation_label: string;
  translate_label: string;
  command_label: string;
  key_choices: { id: string; label: string; modifier: boolean }[];
  /** La touche de dictée par défaut du système (« right_control » ou « right_option »), et celles qui, seules, écoutent tout de suite. */
  default_dictation_id: string;
  /** « Ctrl droit » et « Maj », ou « Option droite » et « ⌘ » : les touches rétablies par « Rétablir ». */
  default_dictation_label: string;
  default_translate_label: string;
  default_command_label: string;
  immediate_keys: string[];
  paths: { data: string; settings: string; models: string; log: string; engines: string | null };
  transcription: string;
  transcription_engine: 'whisper' | 'deepgram';
  cleanup: 'none' | 'llamaCpp' | 'openRouter' | 'lmStudio' | 'ollama';
  cleanup_model: string;
  /** Les sons proposés pour le début et la fin de la dictée. */
  sounds: string[];
  cleanup_label: string;
  llama_log: { bytes: number; hints: string[]; head: string[]; tail: string[] };
  llama_device: string;
  llama_idle_minutes: number;
  pause_media: boolean;
  whisper: { binary: string | null; models: string[]; chosen: string };
  llama: { binary: string | null; models: string[]; chosen: string };
  /** Reformulation de la sélection : possible sur ce système ; la case d'accord est utile (moteur hors de cet ordinateur) ; accord donné. */
  rewrite: { available: boolean; needsConsent: boolean; given: boolean };
  keys: { deepgram: boolean; openrouter: boolean };
  microphones: string[];
  microphone_list: { uid: string; label: string }[];
}

/** Les réglages, avec les clés du fichier de réglages (le format de Scribe). */
export interface Settings {
  language: 'fr' | 'en' | 'multi';
  speakerGender: 'unspecified' | 'masculine' | 'feminine';
  /** Prénom de la salutation ; vide : celui du compte de l'ordinateur. */
  firstName: string;
  transcriptionEngine: 'whisper' | 'deepgram';
  whisperModelPath: string;
  transcriptionModel: string;
  whisperIdleMinutes: number;
  cleanupEngine: 'openRouter' | 'lmStudio' | 'llamaCpp' | 'ollama' | 'none';
  cleanupModel: string;
  llamaCppModel: string;
  llamaDevice: string;
  llamaIdleMinutes: number;
  pauseMedia: boolean;
  listBullet: 'dash' | 'dot' | 'star';
  /** Commandes vocales sur du texte sélectionné : interrupteur, phrases choisies (par langue puis par commande) et raccourcis de gras, italique, souligné. */
  voiceCommandsEnabled: boolean;
  voiceCommands: { fr: Record<string, string[]>; en: Record<string, string[]> };
  /** Identifiants des commandes désactivées une à une (sous l'interrupteur général) ; absent ou vide : toutes actives. */
  disabledCommands: string[];
  customCommands: { id: string; name: string; phrases: string[]; shortcut: string; needsSelection: boolean }[];
  /** Sites de recherche de la personne (« nouvelle recherche <site> … » sous la touche de commande) : adresse https avec {q}. */
  webSearches: { id: string; name: string; url: string; phrases: string[] }[];
  snippets: { id: string; name: string; phrases: string[]; text: string }[];
  /** Applications ouvertes à la voix (liste autorisée, sous la touche de commande) : propre à l'ordinateur, jamais exporté. */
  voiceApps: { id: string; name: string; appId: string; phrases: string[] }[];
  /** Accord pour envoyer la sélection à reformuler à un moteur hors de cet ordinateur (lié au moteur, au modèle et à l'adresse). La page
   *  n'écrit que `true` (l'accord pour le moteur montré, lié par le serveur) ou `false` ; elle lit l'objet rangé, ou `null`. */
  rewriteConsent: { engine: string; model: string; address: string } | null | boolean;
  formatShortcuts: { bold: string; italic: string; underline: string };
  trialText: string;
  trialCandidates: { id: string; engine: string; model: string }[];
  startSoundName: string;
  endSoundName: string;
  soundVolume: number;
  restoreClipboard: boolean;
  skipWhenClean: boolean;
  microphoneUID: string;
  translationLanguage: 'en' | 'es' | 'de' | 'it' | 'pt';
  keepDictationText: boolean;
  typingWPM: number;
  setupDone: boolean;
  overlayEnabled: boolean;
  /** Identifiants des touches de dictée (vide : Ctrl droit) et de la touche de traduction (vide : Maj). */
  dictationKeys: string[];
  translateKeys: string[];
  /** Identifiant de la touche tenue en plus pour passer en commande (vide : celle par défaut du système). */
  commandKeys: string[];
  /** Langue de l'interface : automatique (celle du système), français ou anglais. */
  uiLanguage: 'auto' | 'fr' | 'en';
  theme: 'auto' | 'light' | 'dark';
  /** Chercher toutes les 6 heures s'il existe une nouvelle version. */
  checkUpdates: boolean;
  /** Le curseur du logo de la barre latérale clignote (vrai par défaut) ou reste fixe. */
  animatedLogo?: boolean;
  captureStyle: 'pill' | 'dot' | 'meter';
  capturePosition: 'left' | 'center' | 'right';
  captureColor: CaptureColor;
  captureOpacity: number;
  historyKeepDays?: number | null;
  [key: string]: unknown;
}

export interface Phrase {
  category: string;
  text: string;
  expect: string;
  checks: string[];
}

export interface Timing {
  transcription: number;
  cleanup: number;
  total: number;
  outcome: string;
}

export interface Profile {
  peak: number;
  clipped: number;
  levels: number[];
}

export interface Take {
  ok: boolean;
  error?: string;
  duration: number;
  level: number;
  verdict: 'muet' | 'faible' | 'correct';
  gain: number;
  profile: Profile;
}

/** Les réglages d'un passage du banc, écrits comme on les lit. */
export interface BenchSetup {
  version: string;
  transcription: string;
  cleanup: string;
  mode: string;
  modeDetail: string;
  skipWhenClean: boolean;
  microphone: string;
}

export interface RunSummary {
  id: string;
  date: string;
  setup: BenchSetup;
  summary: string;
  settings_changed: boolean;
  phrases: number;
  checks_passed: number;
  checks_total: number;
  average_total: number | null;
  average_by_outcome: { outcome: string; average: number; count: number }[];
}

export interface DiffWord {
  text: string;
  changed: boolean;
}

export interface ComparedSide {
  pasted: string;
  raw: string;
  timing: Timing;
  checks: { label: string; passed: boolean }[];
}

export interface ComparedRow {
  category: string;
  text: string;
  in_both: boolean;
  changed: boolean;
  improvements: string[];
  regressions: string[];
  older: ComparedSide | null;
  newer: ComparedSide | null;
  older_words: DiffWord[] | null;
  newer_words: DiffWord[] | null;
}

export interface Comparison {
  older: RunSummary;
  newer: RunSummary;
  common: number;
  improved: number;
  regressed: number;
  rows: ComparedRow[];
}

export interface Transcribed {
  ok: boolean;
  error?: string;
  raw: string | null;
  text: string | null;
  timing: Timing | null;
  messages: string[];
  checks: { label: string; passed: boolean }[];
  duration: number;
  level: number;
  gain: number;
  verdict: 'muet' | 'faible' | 'correct';
  profile: Profile;
  setup: BenchSetup;
  /** Banc seulement : l'option « Sauter si déjà propre » aurait-elle sauté le modèle ? */
  would_skip?: boolean;
  /** Banc seulement : ce qui serait collé sans modèle. */
  without_model?: string;
  /** Banc seulement : le texte donné à la mise au propre (pour la comparaison avec le modèle). */
  cleanup_input?: string | null;
  /** Banc seulement : la phrase a été sautée, le modèle doit écrire sa version à côté. */
  compare?: boolean;
}

/** Ce que le modèle a écrit pour une phrase sautée (banc). */
export interface ModelVersion {
  text?: string;
  seconds?: number;
  error?: string;
}

export interface Correction {
  heard: string;
  written: string;
}

export interface Dictionary {
  words: string[];
  corrections: Correction[];
}

export interface ModeExample {
  id: string;
  dictation: string;
  expected: string;
}

export interface CustomMode {
  id: string;
  name: string;
  style: string;
  fidelity: 'literal' | 'light' | 'reformulated' | 'free';
  vouvoie: boolean;
  examples: ModeExample[];
}

export interface BuiltInMode {
  id: string;
  label: string;
  summary: string;
  has_style: boolean;
  original_style: string | null;
  typed_style: string;
  modified: boolean;
  available: boolean;
}

export interface Modes {
  built_in: BuiltInMode[];
  groups: string[][];
  choice: string;
  active_label: string;
}

export interface HistoryItem {
  id: string;
  date: string;
  app: string;
  raw: string;
  text: string;
  has_text: boolean;
  words: number;
  duration: number;
  wpm: number;
  corrections: number;
  translation: string | null;
  timing: { transcription: number; cleanup: number; total: number; outcome: string } | null;
  /** Le mode de reformulation en cours à la dictée (absent : reformulation coupée ou ancienne dictée). */
  mode?: string | null;
}

export interface Stats {
  dictations: number;
  total_words: number;
  today_words: number;
  yesterday_words: number;
  today_change: number | null;
  average_wpm: number;
  time_saved: number;
  streak: number;
  corrections_this_week: number;
  last_days: { day: string; words: number }[];
  top_apps: { name: string; words: number; share: number }[];
  typing_wpm: number;
}

/** Un modèle du dossier de l'appli (page Installation). */
export interface InstalledModel {
  name: string;
  path: string;
  size: number;
  /** « llama », « whisper » ou « partial » (téléchargement interrompu). */
  kind: 'llama' | 'whisper' | 'partial';
  used: boolean;
}

export type DownloadProgress = { state: 'idle' | 'running' | 'done' | 'failed'; done?: number; total?: number; error?: string };

/** Ce que Hugging Face répond à « Chercher » : les fichiers d'un dépôt, ou des dépôts. */
export type HubAnswer =
  | { mode: 'files'; repo: string; kind: 'llama' | 'whisper'; files: { name: string; size: number; note: string | null; downloadable: boolean; id: string }[] }
  | { mode: 'search'; repos: { repo: string; downloads: number; likes: number }[] };
