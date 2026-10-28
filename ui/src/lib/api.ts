/** Le client du serveur local : l'adresse de la page contient le jeton secret, les chemins sont donc relatifs. */

export interface ApiError {
  ok: false;
  error: string;
}

export async function api<T = any>(path: string, body?: unknown): Promise<T> {
  const init: RequestInit =
    body === undefined ? {} : { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) };
  const response = await fetch('api/' + path, init);
  return response.json();
}

export interface Status {
  version: string;
  windows: boolean;
  /** « windows », « macos » ou « linux » : choisit la couleur de la marque. */
  os: string;
  recording: boolean;
  busy: boolean;
  keys_error: string | null;
  hotkey: string;
  /** « Ctrl gauche + F9 » : la dictée et la traduction en vigueur. */
  dictation_label: string;
  translate_label: string;
  key_choices: { id: string; label: string; modifier: boolean }[];
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
  keys: { deepgram: boolean; openrouter: boolean };
  microphones: string[];
  microphone_list: { uid: string; label: string }[];
}

/** Les réglages, avec les clés du fichier de réglages (le format de Scribe). */
export interface Settings {
  language: 'fr' | 'en' | 'multi';
  transcriptionEngine: 'whisper' | 'deepgram';
  whisperModelPath: string;
  cleanupEngine: 'openRouter' | 'lmStudio' | 'llamaCpp' | 'ollama' | 'none';
  cleanupModel: string;
  llamaCppModel: string;
  llamaDevice: string;
  llamaIdleMinutes: number;
  pauseMedia: boolean;
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
  captureStyle: 'pill' | 'dot' | 'meter';
  capturePosition: 'left' | 'center' | 'right';
  captureColor: 'scribe' | 'gray' | 'red' | 'blue' | 'black';
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
  rewrites: boolean;
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
