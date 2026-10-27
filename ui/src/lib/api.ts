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
  recording: boolean;
  busy: boolean;
  keys_error: string | null;
  hotkey: string;
  paths: { data: string; settings: string; models: string; log: string; engines: string | null };
  transcription: string;
  transcription_engine: 'whisper' | 'deepgram';
  cleanup: 'none' | 'llamaCpp' | 'openRouter' | 'other';
  cleanup_model: string;
  cleanup_label: string;
  llama_log: { bytes: number; hints: string[]; head: string[]; tail: string[] };
  llama_device: string;
  llama_idle_minutes: number;
  pause_media: boolean;
  whisper: { binary: string | null; models: string[]; chosen: string };
  llama: { binary: string | null; models: string[]; chosen: string };
  keys: { deepgram: boolean; openrouter: boolean };
  microphones: string[];
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
}
