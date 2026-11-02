/** Les cartes repliées ou dépliées, retenues d'une visite à l'autre (par navigateur) : la clé d'une carte est son identifiant. */
const PREFIX = 'fold:';

/** L'état retenu d'une carte : `true` repliée, `false` dépliée, `null` si l'utilisateur n'a jamais touché à cette carte. */
export function savedFold(key: string): boolean | null {
  try {
    const value = localStorage.getItem(PREFIX + key);
    return value === null ? null : value === '1';
  } catch {
    return null;
  }
}

export function saveFold(key: string, folded: boolean): void {
  try {
    localStorage.setItem(PREFIX + key, folded ? '1' : '0');
  } catch {
    /* le stockage peut être refusé : la carte reste dans l'état de la page */
  }
}
