import { api, type Settings } from './api';
import { status } from './status.svelte';

/** Les réglages du fichier de réglages : lus au démarrage, changés par petits morceaux (le serveur fusionne et enregistre). */
class SettingsStore {
  value = $state<Settings | null>(null);
  error = $state('');

  async load(): Promise<void> {
    this.value = (await api<{ settings: Settings }>('settings')).settings;
  }

  async change(changes: Partial<Settings>): Promise<boolean> {
    const out = await api<{ ok: boolean; error?: string; settings?: Settings }>('settings', changes);
    if (out.ok && out.settings) {
      this.value = out.settings;
      this.error = '';
    } else {
      this.error = out.error ?? 'réglage refusé';
      alert(this.error);
    }
    await status.refresh();
    return !!out.ok;
  }
}

export const settings = new SettingsStore();
