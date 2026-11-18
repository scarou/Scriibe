import { api } from '../../lib/api';
import { settings } from '../../lib/settings.svelte';
import { notice } from '../../lib/dialog.svelte';
import { t, tx } from '../../lib/i18n.svelte';

export type Check = { ok: boolean; phrase?: string; error?: 'too_short' | 'taken' | 'unknown'; other?: string };
export type Site = { id: string; name: string; url: string; phrases: string[] };
export type Snippet = { id: string; name: string; phrases: string[]; text: string };
export type VoiceApp = { id: string; name: string; appId: string; phrases: string[] };
export type Custom = { id: string; name: string; phrases: string[]; shortcut: string; needsSelection: boolean };

/** Les commandes, par groupe, avec leur libellé (traduit à l'affichage). */
export const groups = [
  { title: tx('Ajouter au dictionnaire'), commands: [{ id: 'dictionnaire.ajouter', label: tx('Ajouter au dictionnaire') }] },
  {
    title: tx('Entourer la sélection'),
    commands: [
      { id: 'entourer.parentheses', label: tx('Entre parenthèses') },
      { id: 'entourer.guillemets', label: tx('Entre guillemets') },
      { id: 'entourer.crochets', label: tx('Entre crochets') },
      { id: 'entourer.accolades', label: tx('Entre accolades') },
    ],
  },
  {
    title: tx('Gras, italique, souligné'),
    commands: [
      { id: 'format.gras', label: tx('Gras'), shortcut: 'bold' },
      { id: 'format.italique', label: tx('Italique'), shortcut: 'italic' },
      { id: 'format.souligne', label: tx('Souligné'), shortcut: 'underline' },
    ],
  },
  {
    title: tx('Recherches sur internet'),
    intro: tx('Ces phrases ne marchent que sous la touche de commande : « nouvelle recherche YouTube chats rigolos » ouvre la recherche dans ton navigateur. Dites sans la touche, elles restent du texte.'),
    commands: [
      { id: 'recherche.1', label: tx('Google') },
      { id: 'recherche.2', label: tx('YouTube') },
      { id: 'recherche.3', label: tx('Wikipédia') },
      { id: 'recherche.4', label: tx('DuckDuckGo') },
    ],
  },
  {
    title: tx('Nouveau mail'),
    intro: tx('Ces phrases ne marchent que sous la touche de commande : « nouveau mail » ouvre un message vierge dans ta messagerie, « nouveau mail réunion de lundi » préremplit l’objet. Rien n’est envoyé, et il n’y a pas de destinataire : « nouveau mail à Marie » met « à Marie » en objet. Dites sans la touche, elles restent du texte.'),
    commands: [{ id: 'mail.nouveau', label: tx('Nouveau mail') }],
  },
];
export const labelOf = (id: string) => groups.flatMap((g) => g.commands).find((c) => c.id === id)?.label ?? id;

export const wordsIn = (text: string) => text.trim().split(/\s+/).filter(Boolean).length;

/** Le nom d'une commande de l'utilisateur (perso, site, extrait, application) d'après son identifiant. */
export function nameOf(id: string): string | undefined {
  const v = settings.value;
  const all = [...(v?.customCommands ?? []), ...(v?.webSearches ?? []), ...(v?.snippets ?? []), ...(v?.voiceApps ?? [])] as { id: string; name: string }[];
  return all.find((c) => c.id === id)?.name;
}

export const takenMessage = (other?: string) => t('Cette phrase sert déjà à la commande « {name} ».', { name: nameOf(other ?? '') ?? t(labelOf(other ?? '')) });
export const tooShortMessage = () => t('Une phrase compte au moins deux mots : un mot seul est trop souvent du texte à coller.');
export const snippetPhraseMessage = () => t('Donne une phrase à dire (un mot seul suffit : elle doit être toute la phrase dite).');
export const badShortcutMessage = () => t('Raccourci illisible : écris par exemple « Mod+B » (⌘B sur Mac, Ctrl+B ailleurs), avec Mod ou Ctrl ; Mod+Q (quitter) est refusé.');

export async function shortcutLabel(text: string): Promise<{ ok: boolean; label?: string }> {
  return api<{ ok: boolean; label?: string }>('commands/validate-shortcut', { text });
}

/** La commande `id` est-elle désactivée (son interrupteur propre, sous l'interrupteur général) ? */
export const isOff = (id: string) => (settings.value?.disabledCommands ?? []).includes(id);

/** Active ou désactive la commande `id` : le serveur écrit sous son verrou, puis la page relit les réglages. */
export async function setCommandEnabled(id: string, enabled: boolean): Promise<void> {
  const out = await api<{ ok: boolean; error?: string }>('commands/enabled', { id, enabled });
  if (!out.ok) {
    void notice(out.error ?? t('réglage refusé'));
    return;
  }
  await settings.load();
}
