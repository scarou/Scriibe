/** Le raccourci d'une touche appuyée, écrit comme dans les réglages (« Mod+Shift+X ») : `Mod` est ⌘ sur Mac, Ctrl ailleurs.
 *  `null` tant qu'il manque la lettre ou le chiffre, ou un modificateur autre que Maj (une lettre seule s'écrirait dans le texte). */
export function shortcutOf(
  e: { code: string; metaKey: boolean; ctrlKey: boolean; altKey: boolean; shiftKey: boolean },
  mac: boolean,
): string | null {
  const key = /^Key([A-Z])$/.exec(e.code)?.[1] ?? /^Digit(\d)$/.exec(e.code)?.[1];
  if (!key) return null;
  const parts: string[] = [];
  if (mac ? e.metaKey : e.ctrlKey) parts.push('Mod');
  if (mac && e.ctrlKey) parts.push('Ctrl');
  if (e.altKey) parts.push('Alt');
  if (parts.length === 0) return null;
  if (e.shiftKey) parts.push('Shift');
  parts.push(key);
  return parts.join('+');
}
