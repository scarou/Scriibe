/**
 * La ligne qui dit pourquoi une touche est refusée (page des touches). Le refus du programme commence souvent par le nom de la touche
 * (« AltGr (Alt droit) ne peut pas lancer la dictée… ») : le répéter devant lui donnait « AltGr (Alt droit) — AltGr (Alt droit) ne peut pas… »
 * (enquête C7, docs/ENQUETE_WINDOWS.md). Un refus qui ne nomme pas la touche (« trois touches au plus ») garde son préfixe.
 */
export function refusalLine(
  names: string,
  message: string,
  nameOf: (names: string) => string = (n) => n,
  textOf: (message: string) => string = (m) => m,
): string {
  // Le test se fait sur les textes d'origine (français) : une fois traduits, le nom (partie par partie) et le message (en bloc)
  // ne se ressemblent plus et le nom était écrit deux fois, à moitié en français (enquête Windows, M1).
  if (!names || message.startsWith(names)) return textOf(message);
  return `${nameOf(names)} — ${textOf(message)}`;
}
