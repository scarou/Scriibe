import { describe, expect, it } from 'vitest';
import cases from '../../../fixtures/ui_logic.json';
import {
  dictationZoneClears,
  foldedCardsContains,
  foldedCardsToggle,
  MeterHistory,
  METER_HISTORY_LENGTH,
  overlayOriginX,
  type CapturePosition,
} from './logic';

interface Case {
  id: string;
  name: string;
  fn: string;
  args: Record<string, any>;
  expected: unknown;
  comparison?: string;
}

/** Les cas d'invariant n'ont pas de paire entrée → sortie : un test écrit à la main cite leur identifiant (voir plus bas). */
const handWritten = ['ui_logic-012', 'ui_logic-014'];
/** Cas qui ne s'appliquent pas à l'interface de Scriibe (écart noté dans docs/ECARTS.md). */
const notApplicable: Record<string, string> = {
  'ui_logic-013': 'Réglages en cinq rubriques : Scriibe sépare Modes et Mesures en onglets, et « Apparence » attend la fenêtre d’indicateur',
};

function run(c: Case): unknown {
  const a = c.args;
  switch (c.fn) {
    case 'overlay_origin_x':
      return overlayOriginX(a.position as CapturePosition, a.screen, a.width);
    case 'folded_cards_toggle':
      return foldedCardsToggle(a.list, a.id);
    case 'folded_cards_contains':
      return foldedCardsContains(a.list, a.id);
    case 'dictation_zone_clears':
      return dictationZoneClears(a as never);
    default:
      throw new Error(`fonction inconnue : ${c.fn}`);
  }
}

describe('fixtures/ui_logic.json', () => {
  const all = (cases as { cases: Case[] }).cases;

  for (const c of all.filter((c) => c.fn !== 'invariant')) {
    it(`${c.id} ${c.name}`, () => {
      const got = run(c);
      if (c.comparison === 'same_set') {
        expect(String(got).split(',').sort()).toEqual(String(c.expected).split(',').sort());
      } else {
        expect(got).toEqual(c.expected);
      }
    });
  }

  it('chaque cas d’invariant est repris par un test écrit à la main ou écarté avec sa raison', () => {
    const invariants = all.filter((c) => c.fn === 'invariant').map((c) => c.id);
    for (const id of invariants) expect(handWritten.includes(id) || id in notApplicable, id).toBe(true);
  });

  it('ui_logic-012 vumètre : courbe vide hors écoute, bornée pendant l’écoute, le plus récent à droite', () => {
    const meter = new MeterHistory();
    meter.push(0.9);
    expect(meter.history.length).toBe(0);
    meter.setListening(true);
    for (let i = 0; i < 100; i++) meter.push(i / 100);
    expect(meter.history.length).toBe(METER_HISTORY_LENGTH);
    expect(meter.history.at(-1)).toBe(0.99);
  });
});
