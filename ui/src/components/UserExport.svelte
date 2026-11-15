<script lang="ts">
  import Card from './Card.svelte';
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { t } from '../lib/i18n.svelte';

  /** L'export d'un utilisateur entier (étape 5) : ce qui part, par catégorie, avec une case chacune, avant que le serveur ne demande où enregistrer. */
  interface Preview {
    ok: boolean;
    error?: string;
    name: string;
    first_name: string;
    preferences: string[];
    words: string[];
    corrections: { heard: string; written: string }[];
    modes: string[];
    examples: number;
    styles: string[];
    commands: string[];
    web_searches: { name: string; host: string }[];
  }

  let { id, onclose }: { id: string; onclose: () => void } = $props();

  let preview = $state<Preview | null>(null);
  let failed = $state('');
  // Le prénom et le nom affiché sont décochés par défaut : un fichier qui voyage ne dit pas qui l'a écrit tant qu'on ne l'a pas voulu.
  let take = $state({ identity: false, preferences: true, words: true, corrections: true, modes: true, examples: true, styles: true, commands: true, web_searches: true });
  let busy = $state(false);
  let result = $state('');

  onMount(async () => {
    const out = await api<Preview>('profile/export/preview', { id });
    if (!out.ok) {
      failed = out.error ?? t('Impossible de lire cet utilisateur.');
      return;
    }
    preview = out;
    take = {
      identity: false,
      preferences: out.preferences.length > 0,
      words: out.words.length > 0,
      corrections: out.corrections.length > 0,
      modes: out.modes.length > 0,
      examples: out.examples > 0,
      styles: out.styles.length > 0,
      commands: out.commands.length > 0,
      web_searches: out.web_searches.length > 0,
    };
  });

  /** Quelque chose part-il ? (une case cochée sur une catégorie qui n'est pas vide) */
  let leaves = $derived(
    !!preview &&
      ((take.identity && (!!preview.first_name || !!preview.name)) ||
        (take.preferences && preview.preferences.length > 0) ||
        (take.words && preview.words.length > 0) ||
        (take.corrections && preview.corrections.length > 0) ||
        (take.modes && preview.modes.length > 0) ||
        (take.styles && preview.styles.length > 0) ||
        (take.commands && preview.commands.length > 0) ||
        (take.web_searches && preview.web_searches.length > 0)),
  );

  async function save() {
    busy = true;
    result = '';
    const out = await api<{ ok: boolean; saved?: boolean; path?: string; error?: string }>('profile/export', { id, ...take });
    busy = false;
    result = !out.ok
      ? t(out.error ?? 'enregistrement impossible : l’emplacement choisi n’est pas accessible en écriture')
      : out.saved
        ? t('Enregistré : {path}', { path: out.path ?? '' })
        : t('Export annulé : rien n’a été enregistré.');
  }
</script>

<Card id="utilisateur-export" accent title={preview ? t('Exporter l’utilisateur « {name} »', { name: preview.name }) : t('Exporter un utilisateur')} help="start">
  {#if failed}
    <p class="warn">{failed}</p>
  {:else if !preview}
    <p class="muted">{t('Chargement…')}</p>
  {:else}
    <p class="muted">{t('Voici ce qui partirait dans le fichier. Décoche ce que tu ne veux pas donner. Le fichier n’est pas chiffré et peut contenir des noms propres et des textes libres (corrections, consignes et exemples de modes, phrases de commandes) : relis-le avant de le partager. Il ne contient jamais de clé d’API, d’historique, de statistiques ni de réglage de l’ordinateur (touches, micro, moteurs).')}</p>
    <div class="cat">
      <label><input type="checkbox" bind:checked={take.identity} disabled={busy} /> {t('Prénom et nom affiché')}</label>
      <span class="muted">{preview.first_name ? t('prénom « {first} », nom affiché « {name} »', { first: preview.first_name, name: preview.name }) : t('nom affiché « {name} » (pas de prénom)', { name: preview.name })} · {t('décoché par défaut')}</span>
    </div>
    <div class="cat">
      <label><input type="checkbox" bind:checked={take.preferences} disabled={busy || preview.preferences.length === 0} /> {t('Réglages de personne')}</label>
      <span class="muted">{preview.preferences.length > 0 ? preview.preferences.map((p) => t(p)).join(', ') : t('rien')}</span>
    </div>
    <div class="cat">
      <label><input type="checkbox" bind:checked={take.words} disabled={busy || preview.words.length === 0} /> {t('Mots')}</label>
      <span class="muted">{t('{n} mot(s)', { n: preview.words.length })}</span>
      {#if preview.words.length > 0}<details><summary>{t('Voir la liste')}</summary><p class="list">{preview.words.join(', ')}</p></details>{/if}
    </div>
    <div class="cat">
      <label><input type="checkbox" bind:checked={take.corrections} disabled={busy || preview.corrections.length === 0} /> {t('Corrections')}</label>
      <span class="muted">{t('{n} correction(s)', { n: preview.corrections.length })}</span>
      {#if preview.corrections.length > 0}<details><summary>{t('Voir la liste')}</summary><ul class="list">{#each preview.corrections as c, i (i)}<li>« {c.heard} » → « {c.written} »</li>{/each}</ul></details>{/if}
    </div>
    <div class="cat">
      <label><input type="checkbox" bind:checked={take.modes} disabled={busy || preview.modes.length === 0} /> {t('Modes personnels')}</label>
      <span class="muted">{t('{n} mode(s)', { n: preview.modes.length })}</span>
      {#if preview.modes.length > 0}<details><summary>{t('Voir la liste')}</summary><p class="list">{preview.modes.join(', ')}</p></details>{/if}
      <label class="sub"><input type="checkbox" bind:checked={take.examples} disabled={busy || !take.modes || preview.examples === 0} /> {t('Inclure leurs exemples ({n})', { n: preview.examples })}</label>
    </div>
    <div class="cat">
      <label><input type="checkbox" bind:checked={take.styles} disabled={busy || preview.styles.length === 0} /> {t('Consignes modifiées des modes livrés')}</label>
      <span class="muted">{preview.styles.length > 0 ? preview.styles.map((s) => t(s)).join(', ') : t('rien')}</span>
    </div>
    <div class="cat">
      <label><input type="checkbox" bind:checked={take.commands} disabled={busy || preview.commands.length === 0} /> {t('Commandes et leurs phrases')}</label>
      <span class="muted">{t('{n} commande(s)', { n: preview.commands.length })}</span>
      {#if preview.commands.length > 0}<details><summary>{t('Voir la liste')}</summary><p class="list">{preview.commands.join(', ')}</p></details>{/if}
    </div>
    <div class="cat">
      <label><input type="checkbox" bind:checked={take.web_searches} disabled={busy || preview.web_searches.length === 0} /> {t('Sites de recherche vocale')}</label>
      <span class="muted">{t('{n} site(s)', { n: preview.web_searches.length })}</span>
      {#if preview.web_searches.length > 0}<details><summary>{t('Voir la liste')}</summary><p class="list">{preview.web_searches.map((w) => w.name + ' (' + (w.host || '—') + ')').join(', ')}</p></details>{/if}
    </div>
    <div class="row">
      <button class="primary" onclick={save} disabled={busy || !leaves}>{t('Enregistrer…')}</button>
      <button onclick={onclose} disabled={busy}>{t('Fermer')}</button>
    </div>
    {#if !leaves}<p class="muted">{t('Rien à exporter avec ces cases : coche une catégorie qui n’est pas vide.')}</p>{/if}
    {#if result}<p>{result}</p>{/if}
  {/if}
</Card>

<style>
  .cat { padding: 4px 0; display: flex; flex-wrap: wrap; gap: 4px 12px; align-items: baseline; }
  .sub { flex-basis: 100%; margin-left: 1.6em; color: var(--muted); font-size: .9rem; }
  .list { margin: 4px 0; max-height: 10em; overflow: auto; flex-basis: 100%; }
  ul.list { padding-left: 1.2em; }
  details { flex-basis: 100%; }
  .warn { color: var(--warn); }
</style>
