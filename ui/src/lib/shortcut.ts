/** Le raccourci d'une touche appuyée, écrit comme dans les réglages (« Mod+Shift+X ») : `Mod` est ⌘ sur Mac, Ctrl ailleurs.
 *  `null` tant qu'il manque la lettre ou le chiffre, ou ⌘ ou Ctrl (une lettre seule, avec Maj ou Option, s'écrirait dans le texte), et pour ⌘Q (quitter). */
export function shortcutOf(
  e: { code: string; metaKey: boolean; ctrlKey: boolean; altKey: boolean; shiftKey: boolean },
  mac: boolean,
): string | null {
  const key = /^Key([A-Z])$/.exec(e.code)?.[1] ?? /^Digit(\d)$/.exec(e.code)?.[1];
  if (!key) return null;
  const parts: string[] = [];
  if (mac ? e.metaKey : e.ctrlKey) parts.push('Mod');
  if (mac && e.ctrlKey) parts.push('Ctrl');
  if (parts.length === 0) return null;
  if (key === 'Q' && (mac ? e.metaKey : e.ctrlKey)) return null;
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  parts.push(key);
  return parts.join('+');
}
