/** Durées de veille d'un moteur (Whisper, llama.cpp) : minutes sans dictée avant qu'il soit libéré ; 0 : jamais (gardé en mémoire). */

export const idleChoices = [1, 2, 5, 10, 15, 30, 60, 0];

/** Les durées du menu ; un ancien réglage hors liste (2 h ou 8 h de l'ancien menu de llama.cpp) y reste, lu tel quel, avant « Jamais ». */
export function idleOptions(current: number): number[] {
  return idleChoices.includes(current) ? idleChoices : [...idleChoices.slice(0, -1), current, 0].sort((a, b) => (a || Infinity) - (b || Infinity));
}
