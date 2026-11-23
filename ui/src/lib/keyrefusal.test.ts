import { describe, expect, it } from 'vitest';
import { refusalLine } from './keyrefusal';

describe('la ligne de refus d’une touche', () => {
  it('ne répète pas le nom de la touche quand le refus le dit déjà', () => {
    expect(refusalLine('AltGr (Alt droit)', 'AltGr (Alt droit) ne peut pas lancer la dictée : elle sert à taper')).toBe(
      'AltGr (Alt droit) ne peut pas lancer la dictée : elle sert à taper',
    );
  });

  it('garde le nom devant un refus qui ne le dit pas', () => {
    expect(refusalLine('Ctrl gauche + Maj gauche', 'une seule touche pour traduire')).toBe('Ctrl gauche + Maj gauche — une seule touche pour traduire');
  });

  it('sans touche vue, le refus seul', () => {
    expect(refusalLine('', 'trois touches au plus')).toBe('trois touches au plus');
  });

  it('en anglais, le nom n’est pas écrit deux fois : le test se fait sur le français', () => {
    const nameOf = (n: string) => n.replaceAll('gauche', 'left');
    const textOf = (m: string) => m.replace('choisie deux fois', 'chosen twice').replace('Ctrl gauche', 'Ctrl left');
    expect(refusalLine('Ctrl gauche', 'Ctrl gauche choisie deux fois', nameOf, textOf)).toBe('Ctrl left chosen twice');
    expect(refusalLine('Ctrl gauche + Maj gauche', 'trois touches au plus', nameOf, (m) => 'three keys at most')).toBe('Ctrl left + Maj left — three keys at most');
  });
});
