/**
 * Les boîtes de confirmation et d'avertissement de l'interface. Les boîtes du navigateur affichent « 127.0.0.1:… indique » en titre :
 * celles-ci disent « Scriibe » et reprennent les couleurs de l'appli.
 */
export interface DialogState {
  message: string;
  /** Texte du bouton qui valide ; `null` : simple avertissement, un seul bouton. */
  confirm: string | null;
  danger: boolean;
  resolve: (ok: boolean) => void;
}

class DialogStore {
  current = $state<DialogState | null>(null);

  /** Demande une confirmation ; vrai si la personne valide. Une boîte déjà ouverte est refusée avant que la nouvelle s'ouvre. */
  confirm(message: string, options: { confirm?: string; danger?: boolean } = {}): Promise<boolean> {
    return this.open({ message, confirm: options.confirm ?? 'OK', danger: options.danger ?? false });
  }

  /** Affiche un message (une erreur, un refus) jusqu'à ce qu'on le ferme. */
  notice(message: string): Promise<boolean> {
    return this.open({ message, confirm: null, danger: false });
  }

  private open(state: Omit<DialogState, 'resolve'>): Promise<boolean> {
    this.current?.resolve(false);
    return new Promise((resolve) => {
      this.current = { ...state, resolve };
    });
  }

  answer(ok: boolean): void {
    const open = this.current;
    this.current = null;
    open?.resolve(ok);
  }
}

export const dialog = new DialogStore();

/** `confirm()` du navigateur, en mieux. */
export const confirmBox = (message: string, options?: { confirm?: string; danger?: boolean }) => dialog.confirm(message, options);

/** `alert()` du navigateur, en mieux. */
export const notice = (message: string) => dialog.notice(message);
