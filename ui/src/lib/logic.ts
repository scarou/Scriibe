/** Les petites logiques d'interface reprises de Scribe (`Checks.swift`), testées sans écran par `fixtures/ui_logic.json`. */

export type CapturePosition = 'left' | 'center' | 'right';

/** Marge de l'indicateur au bord de l'écran. */
export const OVERLAY_MARGIN = 12;

/** Nombre de niveaux gardés pour la courbe du vumètre (`OverlayModel.historyLength`). */
export const METER_HISTORY_LENGTH = 44;

/** Abscisse de la fenêtre d'indicateur : à gauche, au centre ou à droite de l'écran. */
export function overlayOriginX(position: CapturePosition, screen: { x: number; width: number }, width: number): number {
  switch (position) {
    case 'left':
      return screen.x + OVERLAY_MARGIN;
    case 'center':
      return screen.x + screen.width / 2 - width / 2;
    case 'right':
      return screen.x + screen.width - OVERLAY_MARGIN - width;
  }
}

/** Les cartes repliées sont gardées dans une liste d'identifiants séparés par des virgules. */
export function foldedCardsContains(list: string, id: string): boolean {
  return list.split(',').includes(id);
}

/** Plie la carte si elle est dépliée, la déplie si elle est pliée ; les autres ne bougent pas. */
export function foldedCardsToggle(list: string, id: string): string {
  const ids = list.split(',').filter((x) => x !== '');
  return (ids.includes(id) ? ids.filter((x) => x !== id) : [...ids, id]).join(',');
}

/** La zone de texte du banc se vide au début d'une dictée du banc : phrase affichée, appli active, enregistrement en cours. */
export function dictationZoneClears(state: { recording: boolean; phrase_shown: boolean; app_active: boolean }): boolean {
  return state.recording && state.phrase_shown && state.app_active;
}

/** La courbe du vumètre : vide hors écoute, bornée pendant l'écoute, le niveau le plus récent à droite. */
export class MeterHistory {
  private values: number[] = [];
  private listening = false;

  get history(): readonly number[] {
    return this.values;
  }

  /** Début ou fin de l'écoute : à chaque début la courbe repart de zéro. */
  setListening(listening: boolean): void {
    this.listening = listening;
    this.values = listening ? new Array<number>(METER_HISTORY_LENGTH).fill(0) : [];
  }

  push(level: number): void {
    if (!this.listening) return;
    this.values = [...this.values, Math.min(1, Math.max(0, level))].slice(-METER_HISTORY_LENGTH);
  }
}
