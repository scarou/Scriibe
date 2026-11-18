import { afterEach, describe, expect, it, vi } from 'vitest';
import { restoreUiState } from './boot';
import { i18n } from './i18n.svelte';
import { uiState } from './uiState';

describe('démarrage d’une fenêtre', () => {
  afterEach(() => {
    vi.restoreAllMocks();
    uiState.state = { folds: {} };
    i18n.restore();
  });

  it('reprend la langue gardée par Scriibe après la lecture des états (page comme pastille)', async () => {
    expect(i18n.choice).toBe('auto');
    vi.spyOn(uiState, 'load').mockImplementation(async () => {
      uiState.state = { folds: {}, uiLanguage: 'en' };
    });
    await restoreUiState();
    expect(i18n.choice).toBe('en');
  });

  it('sans état gardé, la langue du système', async () => {
    vi.spyOn(uiState, 'load').mockImplementation(async () => {
      uiState.state = { folds: {} };
    });
    await restoreUiState();
    expect(i18n.choice).toBe('auto');
  });
});
