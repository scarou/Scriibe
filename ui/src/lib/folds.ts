import { uiState } from './uiState';

/** Les cartes repliées ou dépliées, retenues d'un lancement à l'autre (fichier d'états de Scriibe, voir `uiState`) : la clé d'une carte est son identifiant. */

/** L'état retenu d'une carte : `true` repliée, `false` dépliée, `null` si l'utilisateur n'a jamais touché à cette carte. */
export function savedFold(key: string): boolean | null {
  const folds = uiState.state.folds;
  // `hasOwn` : une clé comme « constructor » ne doit pas rendre ce que l'objet hérite.
  return Object.hasOwn(folds, key) ? folds[key] : null;
}

export function saveFold(key: string, folded: boolean): void {
  uiState.save({ folds: { [key]: folded } });
}
