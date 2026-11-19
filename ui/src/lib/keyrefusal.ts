/**
 * La ligne qui dit pourquoi une touche est refusée (page des touches). Le refus du programme commence souvent par le nom de la touche
 * (« AltGr (Alt droit) ne peut pas lancer la dictée… ») : le répéter devant lui donnait « AltGr (Alt droit) — AltGr (Alt droit) ne peut pas… »
 * (enquête C7, docs/ENQUETE_WINDOWS.md). Un refus qui ne nomme pas la touche (« trois touches au plus ») garde son préfixe.
 */
export function refusalLine(names: string, message: string): string {
  if (!names || message.startsWith(names)) return message;
  return `${names} — ${message}`;
}
