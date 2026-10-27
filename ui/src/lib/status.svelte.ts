import { api, type Status } from './api';

/** L'état de l'installation et des réglages, relu toutes les 8 s et après chaque changement. */
class StatusStore {
  value = $state<Status | null>(null);
  failed = $state(false);

  async refresh(): Promise<void> {
    try {
      this.value = await api<Status>('status');
      this.failed = false;
    } catch {
      this.failed = true;
    }
  }

  start(): void {
    void this.refresh();
    setInterval(() => void this.refresh(), 8000);
  }
}

export const status = new StatusStore();
