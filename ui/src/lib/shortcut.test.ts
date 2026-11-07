import { describe, expect, it } from 'vitest';
import { shortcutOf } from './shortcut';

const press = (code: string, mods: Partial<{ metaKey: boolean; ctrlKey: boolean; altKey: boolean; shiftKey: boolean }> = {}) => ({
  code,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  ...mods,
});

describe('shortcutOf', () => {
  it('writes ⌘ as Mod on a Mac and Ctrl as Mod elsewhere', () => {
    expect(shortcutOf(press('KeyN', { metaKey: true }), true)).toBe('Mod+N');
    expect(shortcutOf(press('KeyN', { ctrlKey: true }), false)).toBe('Mod+N');
    expect(shortcutOf(press('KeyN', { ctrlKey: true }), true)).toBe('Ctrl+N');
  });
  it('orders the modifiers and accepts digits', () => {
    expect(shortcutOf(press('KeyX', { metaKey: true, shiftKey: true }), true)).toBe('Mod+Shift+X');
    expect(shortcutOf(press('Digit1', { metaKey: true, altKey: true }), true)).toBe('Mod+Alt+1');
  });
  it('refuses a key without a usable modifier or without a letter or digit', () => {
    expect(shortcutOf(press('KeyX'), true)).toBeNull();
    expect(shortcutOf(press('KeyX', { shiftKey: true }), true)).toBeNull();
    expect(shortcutOf(press('Enter', { metaKey: true }), true)).toBeNull();
    expect(shortcutOf(press('KeyN', { metaKey: true }), false)).toBeNull();
    expect(shortcutOf(press('KeyB', { altKey: true }), true)).toBeNull();
    expect(shortcutOf(press('KeyQ', { metaKey: true }), true)).toBeNull();
    expect(shortcutOf(press('KeyQ', { ctrlKey: true }), false)).toBeNull();
    expect(shortcutOf(press('KeyW', { metaKey: true }), true)).toBe('Mod+W');
  });
});
