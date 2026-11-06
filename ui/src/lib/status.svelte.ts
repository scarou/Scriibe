import { api, type Status } from './api';
import { guessOs } from './theme';

/** L'état de l'installation et des réglages, relu toutes les 8 s et après chaque changement. */
class StatusStore {
  value = $state<Status | null>(null);
  failed = $state(false);
  /** La dictée en cours, relue chaque seconde par la ligne d'état de la barre latérale. */
  activity = $state<{ recording: boolean; busy: boolean; keys_error: string | null } | null>(null);

  /** Le système : celui du programme dès qu'il a répondu, deviné d'après le navigateur de la fenêtre d'ici là (logo, touches et couleur ne restent pas à Windows pendant l'attente). */
  get os(): string {
    return this.value?.os ?? guessOs(navigator.userAgent);
  }

  async refreshActivity(): Promise<void> {
    try {
      this.activity = await api('activity');
    } catch {
      /* `refresh` signale déjà l'absence de réponse */
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
