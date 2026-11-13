/** La clé qui dit que deux « entendu » sont le même : sans tenir compte des majuscules, des espaces multiples ni de l'apostrophe
 *  droite ou courbe. Même règle que `heard_key` (`crates/core/src/dictionary.rs`), qui dédoublonne à l'enregistrement. */
export function heardKey(heard: string): string {
  return heard.split(/\s+/).filter(Boolean).join(' ').replace(/\u2019/g, "'").toLowerCase();
}

/** Ce que répond `api/dictionary` à un enregistrement : le dictionnaire enregistré, ou `{ok: false, error}` quand le serveur refuse. */
export type SaveResult = { words: string[]; corrections: { heard: string; written: string }[] } | { ok: false; error?: string };

/** Le dictionnaire enregistré, ou `null` si le serveur a refusé (la réponse d'erreur n'est pas un dictionnaire : la poser dans la page
 *  la casserait). */
export function storedDictionary(out: SaveResult): { words: string[]; corrections: { heard: string; written: string }[] } | null {
  return 'words' in out && Array.isArray(out.words) && Array.isArray(out.corrections) ? out : null;
}

/** Le message d'un enregistrement refusé. */
export function saveError(out: SaveResult): string | undefined {
  return 'error' in out ? out.error : undefined;
}

/** Ajoute une correction ; si cet « entendu » existe déjà, sa valeur « écrit » est remplacée (à la même place) au lieu d'ajouter
 *  une seconde ligne qui ne servirait jamais. */
export function withCorrection<T extends { heard: string; written: string }>(list: T[], added: T): T[] {
  const key = heardKey(added.heard);
  const at = list.findIndex((c) => heardKey(c.heard) === key);
  return at >= 0 ? list.map((c, i) => (i === at ? added : c)) : [...list, added];
}
