import { describe, expect, it } from 'vitest';
import { cleanUiState, UiStateCache } from './uiState';
import contract from '../../../crates/platform/tests/data/ui_state_contract.json';

/** Le contrat avec le programme : les mêmes cas sont joués par `merge` de Rust (`crates/platform/src/ui_state.rs`). */
describe('contrat des états d’affichage avec le programme', () => {
  for (const c of contract.cases as { name: string; state: unknown; patch: Record<string, unknown>; expected: Record<string, unknown> }[]) {
    it(c.name, () => {
      const cache = new UiStateCache(async () => undefined);
      cache.state = cleanUiState(c.state);
      cache.save(c.patch as never);
      // Le programme ne garde pas de clé `folds` vide ; la page garde toujours l'objet.
      const { folds, ...rest } = cache.state;
      const stored = Object.keys(folds).length ? { folds, ...rest } : rest;
      expect(stored).toEqual(c.expected);
    });
  }
});
