/**
 * Quand la fenêtre de Scriibe a le focus, le crochet clavier du système ne reçoit rien (S06, cause inconnue) : la page relaie elle-même
 * ses touches au programme (`api/keys/page`), qui les traite comme celles du crochet. Les répétitions d'une touche tenue sont ignorées,
 * et la perte du focus relâche ce qui était tenu (le relâchement ne serait jamais vu).
 */
export type KeyRelay = {
  down: (e: { code: string; repeat: boolean }) => void;
  up: (e: { code: string }) => void;
  blur: () => void;
};

export function createKeyRelay(send: (body: { code?: string; down?: boolean; release?: boolean }) => Promise<unknown>): KeyRelay {
  const pressed = new Set<string>();
  // Les messages partent l'un après l'autre, dans l'ordre des touches.
  let sending: Promise<unknown> = Promise.resolve();
  const post = (body: { code?: string; down?: boolean; release?: boolean }) => {
    sending = sending.then(() => send(body)).catch(() => {});
  };
  return {
    down(e) {
      if (e.repeat || pressed.has(e.code)) return;
      pressed.add(e.code);
      post({ code: e.code, down: true });
    },
    up(e) {
      if (!pressed.delete(e.code)) return;
      post({ code: e.code, down: false });
    },
    blur() {
      if (pressed.size === 0) return;
      pressed.clear();
      post({ release: true });
    },
  };
}
