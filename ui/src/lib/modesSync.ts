/** `shown` : le choix que la page affiche à cet instant (comparé à chaque passage, pour ne rien rater si la page l'a changé entre-temps).
 * Le mode peut être changé hors de la page (menu de l'icône) : la page le relit à intervalles réguliers et ne réagit que si le choix a changé. */
export function watchModes(
  load: () => Promise<{ choice: string; active_label: string }>,
  apply: (modes: { choice: string; active_label: string }) => void,
  everyMs: number,
  shown: () => string,
): () => void {
  let busy = false;
  const timer = setInterval(async () => {
    if (busy) return;
    busy = true;
    try {
      const latest = await load();
      if (latest.choice !== shown()) {
        apply(latest);
      }
    } catch {
      // Serveur occupé ou fenêtre en veille : on réessaie au prochain passage.
    } finally {
      busy = false;
    }
  }, everyMs);
  return () => clearInterval(timer);
}
