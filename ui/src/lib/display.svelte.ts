import * as format from './format';
import { i18n } from './i18n.svelte';

/** Les formats de `format.ts` dans la langue de l'interface (`format.ts` reste en français : le rapport l'utilise). Lire la langue relance l'affichage. */
export const fmt = (n: number) => format.fmt(n, i18n.lang);
export const mo = (bytes: number) => format.mo(bytes, i18n.lang);
export const taille = (bytes: number) => format.taille(bytes, i18n.lang);
export const compte = (n: number) => format.compte(n, i18n.lang);
export const when = (iso: string) => format.when(iso, i18n.lang);
