import { api } from './api';

/**
 * Les états d'affichage retenus d'un lancement à l'autre : cartes repliées, case « Comparer avec le modèle » du banc, hauteur des cartes
 * du Dictionnaire, copie de la langue de l'interface. Ils sont gardés par Scriibe (fichier `ui-state.json` du dossier de données) et non dans
 * le navigateur : le serveur local prend un port libre à chaque lancement, et le stockage du navigateur est rangé par adresse et port.
 *
 * L'état est lu une fois avant d'afficher la page (`load`) puis gardé en mémoire : les lectures restent synchrones. Les changements sont
 * appliqués tout de suite en mémoire et envoyés au programme en arrière-plan, sans jamais afficher d'erreur (sans réponse, la page garde
 * ses états jusqu'à sa fermeture).
 */

export type UiLanguageChoice = 'fr' | 'en' | 'auto';

export interface UiState {
  /** Carte repliée (`true`) ou dépliée (`false`), par identifiant de carte ; absente : jamais touchée. */
  folds: Record<string, boolean>;
  benchCompare?: boolean;
  dictionaryHeight?: number;
  uiLanguage?: UiLanguageChoice;
}

/** Un changement : `null` retire une carte de `folds` ou la hauteur. */
export interface UiStatePatch {
  folds?: Record<string, boolean | null>;
  benchCompare?: boolean;
  dictionaryHeight?: number | null;
  uiLanguage?: UiLanguageChoice;
}

type Send = (path: string, body?: unknown) => Promise<unknown>;

/** Ne garde de ce que dit le programme que des valeurs du bon type (le programme les a déjà vérifiées : ceinture et bretelles). */
export function cleanUiState(raw: unknown): UiState {
  const state: UiState = { folds: {} };
  if (typeof raw !== 'object' || raw === null || Array.isArray(raw)) return state;
  const r = raw as Record<string, unknown>;
  if (typeof r.folds === 'object' && r.folds !== null && !Array.isArray(r.folds)) {
    for (const [id, v] of Object.entries(r.folds)) if (typeof v === 'boolean') state.folds[id] = v;
  }
  if (typeof r.benchCompare === 'boolean') state.benchCompare = r.benchCompare;
  if (typeof r.dictionaryHeight === 'number' && Number.isFinite(r.dictionaryHeight)) state.dictionaryHeight = r.dictionaryHeight;
  if (r.uiLanguage === 'fr' || r.uiLanguage === 'en' || r.uiLanguage === 'auto') state.uiLanguage = r.uiLanguage;
  return state;
}

function applyPatch(state: UiState, patch: UiStatePatch): void {
  for (const [id, v] of Object.entries(patch.folds ?? {})) {
    if (v === null) delete state.folds[id];
    else state.folds[id] = v;
  }
  if (patch.benchCompare !== undefined) state.benchCompare = patch.benchCompare;
  if (patch.dictionaryHeight === null) delete state.dictionaryHeight;
  else if (patch.dictionaryHeight !== undefined) state.dictionaryHeight = patch.dictionaryHeight;
  if (patch.uiLanguage !== undefined) state.uiLanguage = patch.uiLanguage;
}

function mergePatches(into: UiStatePatch, next: UiStatePatch): UiStatePatch {
  return { ...into, ...next, folds: { ...into.folds, ...next.folds } };
}

export class UiStateCache {
  state: UiState = { folds: {} };
  private pending: UiStatePatch | null = null;
  private timer: ReturnType<typeof setTimeout> | undefined;
  /** Les envois partent l'un après l'autre, dans l'ordre. */
  private chain: Promise<unknown> = Promise.resolve();

  /** `sendLeaving` : l'envoi de la dernière chance, qui survit à la fermeture de la page (par défaut le même que les autres). */
  constructor(
    private send: Send,
    private sendLeaving: Send = send,
  ) {}

  /** Lit l'état gardé par le programme. Sans réponse, réponse illisible ou réponse qui tarde plus de `timeoutMs` : états par défaut,
   * sans erreur (la page ne doit jamais attendre ce confort pour s'afficher) ; une réponse tardive est ignorée. */
  async load(timeoutMs = 2000): Promise<void> {
    const late = Symbol('late');
    let timer: ReturnType<typeof setTimeout> | undefined;
    const timeout = new Promise<typeof late>((resolve) => {
      timer = setTimeout(() => resolve(late), timeoutMs);
    });
    try {
      const raw = await Promise.race([this.send('ui-state'), timeout]);
      this.state = raw === late ? { folds: {} } : cleanUiState(raw);
    } catch {
      this.state = { folds: {} };
    } finally {
      clearTimeout(timer);
    }
  }

  /** Change l'état en mémoire tout de suite, puis l'envoie au programme (au bout de `delayMs` si on en donne un : les changements
   * qui se suivent sont alors envoyés ensemble, pour un geste continu comme le glissement d'une poignée). */
  save(patch: UiStatePatch, delayMs = 0): void {
    applyPatch(this.state, patch);
    this.pending = this.pending ? mergePatches(this.pending, patch) : patch;
    if (this.timer !== undefined) clearTimeout(this.timer);
    this.timer = undefined;
    if (delayMs > 0) this.timer = setTimeout(() => this.flush(), delayMs);
    else this.flush();
  }

  /** Envoie tout de suite ce qui attend. */
  flush(): void {
    if (this.timer !== undefined) clearTimeout(this.timer);
    this.timer = undefined;
    const patch = this.pending;
    this.pending = null;
    if (!patch) return;
    this.chain = this.chain.then(() => this.send('ui-state', patch)).catch(() => undefined);
  }

  /** La page se ferme ou se recharge : ce qui attend encore (un changement différé, une hauteur glissée) part tout de suite, sans attendre
   * les envois en cours, par une requête qui survit à la fermeture (M2 de la relecture). Sans rien en attente : aucun envoi. */
  leave(): void {
    if (this.timer !== undefined) clearTimeout(this.timer);
    this.timer = undefined;
    const patch = this.pending;
    this.pending = null;
    if (!patch) return;
    try {
      void Promise.resolve(this.sendLeaving('ui-state', patch)).catch(() => undefined);
    } catch {
      /* la page se ferme : rien à afficher */
    }
  }
}

export const uiState = new UiStateCache(api, (path, body) => api(path, body, { keepalive: true }));
