import { describe, expect, it } from 'vitest';
import { createKeyRelay } from './keyrelay';

function rig() {
  const sent: object[] = [];
  const relay = createKeyRelay(async (body) => void sent.push(body));
  const flush = () => new Promise((resolve) => setTimeout(resolve, 0));
  return { sent, relay, flush };
}

describe('le relais des touches de la fenêtre', () => {
  it('envoie l’appui et le relâchement, dans l’ordre', async () => {
    const { sent, relay, flush } = rig();
    relay.down({ code: 'ControlRight', repeat: false });
    relay.up({ code: 'ControlRight' });
    await flush();
    expect(sent).toEqual([
      { code: 'ControlRight', down: true },
      { code: 'ControlRight', down: false },
    ]);
  });

  it('ignore la répétition d’une touche tenue', async () => {
    const { sent, relay, flush } = rig();
    relay.down({ code: 'ControlRight', repeat: false });
    relay.down({ code: 'ControlRight', repeat: true });
    relay.down({ code: 'ControlRight', repeat: false });
    await flush();
    expect(sent).toHaveLength(1);
  });

  // Bug Windows de l'enquête C7 (docs/ENQUETE_WINDOWS.md) : la dictée a pu commencer par le crochet du système avant que la fenêtre prenne
  // le focus ; le relâchement vu seulement par la page est le seul qui reste, il est relayé (le programme l'ignore s'il ne sert à rien).
  it('relaie le relâchement d’une touche appuyée avant la fenêtre quand la dictée est en cours', async () => {
    const { sent, relay, flush } = rig();
    relay.up({ code: 'ControlRight' });
    await flush();
    expect(sent).toEqual([{ code: 'ControlRight', down: false }]);
  });

  it('relâche ce qui est tenu quand la fenêtre perd le focus, une seule fois', async () => {
    const { sent, relay, flush } = rig();
    relay.down({ code: 'ControlRight', repeat: false });
    relay.blur();
    relay.blur();
    await flush();
    expect(sent).toEqual([{ code: 'ControlRight', down: true }, { release: true }]);
    // La touche relâchée ailleurs n'est plus tenue : la rappuyer compte de nouveau.
    relay.down({ code: 'ControlRight', repeat: false });
    await flush();
    expect(sent).toHaveLength(3);
  });
});
