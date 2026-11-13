/** La clé qui dit que deux « entendu » sont le même : sans tenir compte des majuscules, des espaces multiples ni de l'apostrophe
 *  droite ou courbe. Même règle que `heard_key` (`crates/core/src/dictionary.rs`), qui dédoublonne à l'enregistrement. */
export function heardKey(heard: string): string {
  return heard.split(/\s+/).filter(Boolean).join(' ').replace(/\u2019/g, "'").toLowerCase();
}

/** Ajoute une correction ; si cet « entendu » existe déjà, sa valeur « écrit » est remplacée (à la même place) au lieu d'ajouter
 *  une seconde ligne qui ne servirait jamais. */
export function withCorrection<T extends { heard: string; written: string }>(list: T[], added: T): T[] {
  const key = heardKey(added.heard);
  const at = list.findIndex((c) => heardKey(c.heard) === key);
  return at >= 0 ? list.map((c, i) => (i === at ? added : c)) : [...list, added];
}
