import { api, type Status } from './api';
import { guessOs } from './theme';
import { nav } from './nav.svelte';

/** L'état de l'installation et des réglages, relu toutes les 8 s et après chaque changement. */
class StatusStore {
  value = $state<Status | null>(null);
  failed = $state(false);
  /** La dictée en cours, relue chaque seconde par la ligne d'état de la barre latérale. */
  activity = $state<{ recording: boolean; busy: boolean; keys_error: string | null; prefill?: boolean } | null>(null);
  private fetchingPrefill = false;

  /** Le système : celui du programme dès qu'il a répondu, deviné d'après le navigateur de la fenêtre d'ici là (logo, touches et couleur ne restent pas à Windows pendant l'attente). */
  get os(): string {
    return this.value?.os ?? guessOs(navigator.userAgent);
  }

  async refreshActivity(): Promise<void> {
    try {
      this.activity = await api('activity');
      if (this.activity?.prefill) void this.takePrefill();
    } catch {
      /* `refresh` signale déjà l'absence de réponse */
    }
  }

  /** Une commande vocale a laissé le texte sélectionné : on le prend (une seule fois) et on ouvre le Dictionnaire sur « Entendu → écrit ». */
  private async takePrefill(): Promise<void> {
    if (this.fetchingPrefill) return;
    this.fetchingPrefill = true;
    try {
      const { heard } = await api<{ heard: string | null }>('prefill', {});
      if (heard) {
        nav.dictionaryPrefill = heard;
        await nav.go('dictionary', 'dictionary-corrections-entendu-ecrit');
      }
    } catch {
      /* la demande expire d'elle-même côté programme */
    } finally {
      this.fetchingPrefill = false;
    }
  }

  async refresh(): Promise<void> {
    try {
      this.value = await api<Status>('status');
      if (this.value.os) document.documentElement.dataset.os = this.value.os;
      this.failed = false;
    } catch {
      this.failed = true;
    }
  }

  start(): void {
    void this.refresh();
    setInterval(() => void this.refresh(), 8000);
    void this.refreshActivity();
    setInterval(() => void this.refreshActivity(), 1000);
  }
}

export const status = new StatusStore();
