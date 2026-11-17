<script lang="ts">
  import Card from './Card.svelte';
  import { onDestroy, onMount } from 'svelte';
  import { api } from '../lib/api';
  import { t } from '../lib/i18n.svelte';

  /** L'import d'un utilisateur entier (étape 5) : le serveur lit et contrôle le fichier, la page montre l'aperçu ; rien n'est écrit avant l'accord, et
   * l'import crée toujours un NOUVEL utilisateur (il ne remplace jamais celui en place). */
  interface Preview {
    ok: boolean;
    cancelled?: boolean;
    ask_path?: boolean;
    error?: string;
    token: string;
    name: string | null;
    first_name: string | null;
    preferences: string[];
    hidden_modes: number;
    words: number;
    corrections: number;
    modes: { name: string; fidelity: string; examples: number; added_as: string }[];
    styles: string[];
    commands: { name: string; phrases: string[]; dropped: string[]; shortcut: string; problem: 'no_name' | 'bad_shortcut' | 'no_phrase' | null }[];
    web_searches: { name: string; host: string | null; phrases: string[] }[];
    snippets: { name: string; phrases: string[]; text: string; usable: boolean }[];
    suggested_name: string;
  }

  let { onclose, onchanged, onuse }: { onclose: () => void; onchanged: () => void; onuse: (id: string) => void } = $props();

  let preview = $state<Preview | null>(null);
  let askPath = $state(false);
  let path = $state('');
  let busy = $state(true);
  let message = $state('');
  let name = $state('');
  let take = $state({ identity: true, preferences: true, words: true, corrections: true, modes: true, examples: true, styles: true, commands: true, web_searches: false, snippets: false });
  let created = $state<{ id: string; name: string } | null>(null);

  const problem = (p: string | null) =>
    p === 'bad_shortcut' ? t('raccourci illisible : cette commande ne sera pas importée') : p === 'no_phrase' ? t('plus aucune phrase utilisable : cette commande ne sera pas importée') : t('sans titre : cette commande ne sera pas importée');
  const fidelityName = (f: string) => ({ literal: t('Littéral'), light: t('Léger'), reformulated: t('Reformulé'), free: t('Libre') })[f] ?? f;

  async function read(chosen?: string) {
    busy = true;
    message = '';
    const out = await api<Preview>('profile/import/read', chosen ? { path: chosen } : {});
    busy = false;
    if (out.ok && out.cancelled) {
      onclose();
      return;
    }
    if (out.ok && out.token) {
      askPath = false;
      preview = out;
      name = out.suggested_name;
      take = { identity: !!(out.name || out.first_name), preferences: out.preferences.length > 0 || out.hidden_modes > 0, words: out.words > 0, corrections: out.corrections > 0, modes: out.modes.length > 0, examples: true, styles: out.styles.length > 0, commands: out.commands.some((c) => !c.problem), web_searches: false, snippets: false };
    } else if (out.ask_path) askPath = true;
    else message = t(out.error ?? 'le fichier n’a pas pu être lu');
  }

  async function apply() {
    if (!preview) return;
    busy = true;
    const out = await api<{ ok: boolean; error?: string; created?: { id: string; name: string }; outcome?: { words: number; corrections: number; modes: string[]; commands: string[]; commands_skipped: number; web_searches: string[]; web_searches_skipped: number; snippets: string[]; snippets_skipped: number } }>('profile/import/apply', {
      token: preview.token,
      name,
      include: take,
    });
    busy = false;
    if (!out.ok || !out.created || !out.outcome) {
      message = t(out.error ?? 'le fichier n’a pas pu être lu');
      return;
    }
    created = out.created;
    const o = out.outcome;
    message = t('Utilisateur « {name} » créé : {words} mot(s), {corrections} correction(s), {modes} mode(s), {commands} commande(s).', { name: out.created.name, words: o.words, corrections: o.corrections, modes: o.modes.length, commands: o.commands.length });
    if (o.commands_skipped > 0) message += ' ' + t('{n} commande(s) écartée(s) (raccourci illisible ou phrases déjà prises).', { n: o.commands_skipped });
    if (o.web_searches.length > 0) message += ' ' + t('{n} site(s) de recherche vocale.', { n: o.web_searches.length });
    if (o.snippets.length > 0) message += ' ' + t('{n} extrait(s) vocal(aux).', { n: o.snippets.length });
    if (o.snippets_skipped > 0) message += ' ' + t('{n} extrait(s) écarté(s) (texte vide ou trop long, ou phrases déjà prises).', { n: o.snippets_skipped });
    if (o.web_searches_skipped > 0) message += ' ' + t('{n} site(s) de recherche écarté(s) (adresse refusée ou phrases déjà prises).', { n: o.web_searches_skipped });
    preview = null;
    onchanged();
  }

  async function cancel() {
    if (preview && !created) await api('profile/import/cancel', {});
    onclose();
  }

  onMount(() => void read());
  onDestroy(() => {
    if (preview && !created) void api('profile/import/cancel', {});
  });
</script>

<Card id="utilisateur-import" accent title={t('Importer un utilisateur')} help="profile">
  {#if askPath}
    <p>{t('Sur ce système, indique le chemin du fichier .scriibe.json à importer :')}</p>
    <div class="row"><input type="text" class="grow" bind:value={path} placeholder="/home/…/scriibe-utilisateur.scriibe.json" disabled={busy} />
      <button class="primary" disabled={busy || !path.trim()} onclick={() => read(path)}>{t('Lire le fichier')}</button></div>
  {:else if preview}
    <p>{t('Ce fichier crée un nouvel utilisateur. Rien n’est modifié tant que tu n’as pas cliqué sur « Importer », et l’utilisateur actuel n’est jamais touché. Décoche ce que tu ne veux pas reprendre, et relis ce qui vient de quelqu’un d’autre : une correction remplace un mot par un autre dans toutes les dictées, une consigne est donnée au modèle, une commande appuie un raccourci de l’appli.')}</p>
    <label class="field">{t('Nom du nouvel utilisateur :')}
      <input type="text" bind:value={name} maxlength="40" disabled={busy} />
    </label>
    <p class="muted">{t('Si ce nom est déjà pris, Scriibe ajoute un numéro.')}</p>
    {#if preview.name || preview.first_name}
      <div class="cat"><label><input type="checkbox" bind:checked={take.identity} disabled={busy} /> {t('Prénom')}</label>
        <span class="muted">{preview.first_name ? '« ' + preview.first_name + ' »' : t('rien')}</span></div>
    {/if}
    {#if preview.preferences.length > 0 || preview.hidden_modes > 0}
      <div class="cat"><label><input type="checkbox" bind:checked={take.preferences} disabled={busy} /> {t('Réglages de personne')}</label>
        <span class="muted">{preview.preferences.map((p) => t(p)).join(', ')}{preview.hidden_modes > 0 ? (preview.preferences.length > 0 ? ', ' : '') + t('modes masqués du menu') : ''}</span></div>
    {/if}
    {#if preview.words > 0}
      <div class="cat"><label><input type="checkbox" bind:checked={take.words} disabled={busy} /> {t('Mots')}</label> <span class="muted">{t('{n} mot(s)', { n: preview.words })}</span></div>
    {/if}
    {#if preview.corrections > 0}
      <div class="cat"><label><input type="checkbox" bind:checked={take.corrections} disabled={busy} /> {t('Corrections')}</label> <span class="muted">{t('{n} correction(s)', { n: preview.corrections })}</span></div>
    {/if}
    {#if preview.modes.length > 0}
      <div class="cat"><label><input type="checkbox" bind:checked={take.modes} disabled={busy} /> {t('Modes personnels')}</label> <span class="muted">{t('{n} mode(s)', { n: preview.modes.length })}</span>
        <ul class="list">{#each preview.modes as m, i (i)}<li><b>{m.added_as}</b> <span class="muted">{fidelityName(m.fidelity)} · {t('{n} exemple(s)', { n: m.examples })}</span></li>{/each}</ul>
        <label class="sub"><input type="checkbox" bind:checked={take.examples} disabled={busy || !take.modes} /> {t('Reprendre leurs exemples')}</label></div>
    {/if}
    {#if preview.styles.length > 0}
      <div class="cat"><label><input type="checkbox" bind:checked={take.styles} disabled={busy} /> {t('Consignes modifiées des modes livrés')}</label> <span class="muted">{preview.styles.map((s) => t(s)).join(', ')}</span></div>
    {/if}
    {#if preview.commands.length > 0}
      <div class="cat"><label><input type="checkbox" bind:checked={take.commands} disabled={busy} /> {t('Commandes et leurs phrases')}</label> <span class="muted">{t('{n} commande(s)', { n: preview.commands.length })}</span>
        <ul class="list">{#each preview.commands as c, i (i)}
          <li><b>{c.name || t('Commande sans titre')}</b> <span class="muted">{c.shortcut}</span>
            {#if c.problem}<br /><span class="warn">{problem(c.problem)}</span>{/if}
            {#if c.dropped.length > 0}<br /><span class="warn">{t('Phrase(s) écartée(s) (déjà prises ou trop courtes) : {list}', { list: c.dropped.map((p) => '« ' + p + ' »').join(' ') })}</span>{/if}</li>
        {/each}</ul></div>
    {/if}
    {#if preview.web_searches.length > 0}
      <div class="cat"><label><input type="checkbox" bind:checked={take.web_searches} disabled={busy || !preview.web_searches.some((w) => w.host)} /> {t('Sites de recherche vocale')}</label> <span class="muted">{t('{n} site(s)', { n: preview.web_searches.length })}</span>
        <p class="warn">{t('Décoché par défaut : un site importé ouvre son adresse quand tu dis sa phrase, sous la touche de commande. Vérifie l’hôte de chacun.')}</p>
        <ul class="list">{#each preview.web_searches as w, i (i)}
          <li><b>{w.name}</b> <span class="muted">{w.host ? t('Ouvre {host}', { host: w.host }) : ''}</span>
            {#if !w.host}<br /><span class="warn">{t('adresse refusée : ce site ne sera pas importé')}</span>{/if}
            <br /><span class="muted">{w.phrases.map((p) => '« ' + p + ' »').join(' ')}</span></li>
        {/each}</ul></div>
    {/if}
    {#if preview.snippets.length > 0}
      <div class="cat"><label><input type="checkbox" bind:checked={take.snippets} disabled={busy || !preview.snippets.some((x) => x.usable)} /> {t('Extraits vocaux')}</label> <span class="muted">{t('{n} extrait(s)', { n: preview.snippets.length })}</span>
        <p class="warn">{t('Décoché par défaut : un extrait importé est collé quand tu dis sa phrase, sous la touche de commande. Lis le texte de chacun avant de cocher.')}</p>
        <ul class="list">{#each preview.snippets as x, i (i)}
          <li><b>{x.name}</b>
            {#if !x.usable}<br /><span class="warn">{t('texte vide ou trop long, ou phrases déjà prises : cet extrait ne sera pas importé')}</span>{/if}
            <br /><span class="muted">{x.phrases.map((p) => '« ' + p + ' »').join(' ')}</span>
            <pre class="snippetpreview">{x.text}</pre></li>
        {/each}</ul></div>
    {/if}
    <div class="row">
      <button class="primary" onclick={apply} disabled={busy || !name.trim()}>{t('Importer')}</button>
      <button onclick={cancel} disabled={busy}>{t('Annuler')}</button>
    </div>
  {:else if busy && !created}
    <p class="muted">{t('Chargement…')}</p>
  {/if}
  {#if message}<p class:warn={!created}>{message}</p>{/if}
  {#if created}
    <div class="row">
      <button class="primary" onclick={() => onuse(created!.id)}>{t('Passer à cet utilisateur')}</button>
      <button onclick={onclose}>{t('Fermer')}</button>
    </div>
  {:else if !preview && !busy}
    <div class="row"><button onclick={onclose}>{t('Fermer')}</button></div>
  {/if}
</Card>

<style>
  .field { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
  .cat { padding: 4px 0; display: flex; flex-wrap: wrap; gap: 4px 12px; align-items: baseline; }
  .sub { flex-basis: 100%; margin-left: 1.6em; color: var(--muted); font-size: .9rem; }
  .snippetpreview { margin: 2px 0 6px; padding: 6px 8px; white-space: pre-wrap; word-break: break-word; max-height: 8em; overflow: auto; background: var(--soft, rgba(127,127,127,.12)); border-radius: 4px; font: inherit; }
  .list { margin: 4px 0; padding-left: 1.2em; flex-basis: 100%; }
  .grow { flex: 1; min-width: 10em; }
  .warn { color: var(--warn); }
</style>
