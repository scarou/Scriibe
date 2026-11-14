<script lang="ts">
  import Card from './Card.svelte';
  import UserExport from './UserExport.svelte';
  import UserImport from './UserImport.svelte';
  import { confirmBox, notice } from '../lib/dialog.svelte';
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { t } from '../lib/i18n.svelte';

  /** Les utilisateurs (étape 4c) : une personne = un dictionnaire, des modes, des commandes, un historique et des statistiques. */
  interface Listed {
    ok: boolean;
    error?: string;
    active: string | null;
    profiles: { id: string; name: string }[];
    max_name: number;
  }

  let list = $state<Listed | null>(null);
  let busy = $state(false);
  let newName = $state('');
  let copy = $state(false);
  /** La ligne dont le nom est en cours de modification. */
  let renaming = $state<{ id: string; name: string } | null>(null);
  /** L'utilisateur dont on prépare l'export, et le panneau d'import (un seul des deux à la fois). */
  let exportId = $state<string | null>(null);
  let importOpen = $state(false);

  async function load() {
    list = await api<Listed>('profiles');
  }

  /** Une demande au serveur : rend la réponse si elle a réussi, sinon montre la raison. */
  async function ask<T extends { ok: boolean; error?: string; list?: Listed }>(path: string, body: unknown): Promise<T | null> {
    busy = true;
    try {
      const out = await api<T>(path, body);
      if (!out.ok) {
        void notice(out.error ?? t('Impossible de faire cela.'));
        return null;
      }
      if (out.list) list = out.list;
      return out;
    } finally {
      busy = false;
    }
  }

  async function create() {
    const out = await ask('profiles/create', { name: newName, copy });
    if (out) {
      newName = '';
      copy = false;
    }
  }

  async function saveName() {
    if (!renaming) return;
    if (await ask('profiles/rename', { id: renaming.id, name: renaming.name })) renaming = null;
  }

  async function use(id: string) {
    // Tout ce que la page montre (dictionnaire, modes, historique, statistiques) est celui de l'utilisateur actif : on recharge.
    if (await ask('profiles/switch', { id })) location.reload();
  }

  async function remove(id: string, name: string) {
    const sure = await confirmBox(
      t('Mettre l’utilisateur « {name} » de côté ? Son dictionnaire, ses modes, son historique et ses statistiques ne seront plus utilisés. Scriibe ne les efface pas : le dossier est déplacé dans « utilisateurs-supprimes », dans le dossier de données, d’où tu peux le reprendre à la main. Pour en garder une copie portable (sans l’historique), annule et clique d’abord sur « Exporter… » sur sa ligne.', { name }),
      { confirm: t('Mettre de côté'), danger: true },
    );
    if (!sure) return;
    const out = await ask<{ ok: boolean; kept_in: string }>('profiles/delete', { id, confirmed: true });
    if (out) void notice(t('« {name} » est mis de côté dans {folder}.', { name, folder: out.kept_in }));
  }

  function openExport(id: string) {
    importOpen = false;
    exportId = id;
  }

  function openImport() {
    exportId = null;
    importOpen = true;
  }

  onMount(() => {
    void load();
  });
</script>

<Card id="utilisateurs" title={t('Utilisateurs')} help="start" intro={t('Chaque utilisateur a son dictionnaire, ses modes personnels, ses commandes, son historique et ses statistiques, et ses réglages de personne (prénom, langue, mode choisi). Les touches, le micro, les moteurs et les clés restent ceux de l’ordinateur. Avec deux utilisateurs ou plus, le menu de l’icône propose aussi de passer de l’un à l’autre.')}>
  {#if !list}
    <p class="muted">{t('Chargement…')}</p>
  {:else if !list.ok}
    <p class="warn">{list.error}</p>
  {:else if list.active === null}
    <p class="muted">{t('Les utilisateurs ne sont pas disponibles : le rangement par utilisateur n’a pas pu se faire au lancement (le journal en dit la raison). Relance Scriibe.')}</p>
  {:else}
    <ul class="users">
      {#each list.profiles as p (p.id)}
        {@const active = p.id.toLowerCase() === list.active.toLowerCase()}
        <li class:active>
          {#if renaming?.id === p.id}
            <input type="text" class="grow" bind:value={renaming.name} maxlength={list.max_name} disabled={busy} aria-label={t('Nom de l’utilisateur')}
              onkeydown={(e) => { if (e.key === 'Enter') void saveName(); else if (e.key === 'Escape') renaming = null; }} />
            <button class="primary" onclick={saveName} disabled={busy || !renaming.name.trim()}>{t('Enregistrer')}</button>
            <button onclick={() => (renaming = null)} disabled={busy}>{t('Annuler')}</button>
          {:else}
            <span class="name"><b>{p.name}</b>{#if active} <span class="tag">{t('actif')}</span>{/if}</span>
            {#if !active}<button onclick={() => use(p.id)} disabled={busy}>{t('Utiliser')}</button>{/if}
            <button onclick={() => openExport(p.id)} disabled={busy}>{t('Exporter…')}</button>
            <button onclick={() => (renaming = { id: p.id, name: p.name })} disabled={busy}>{t('Renommer')}</button>
            <button onclick={() => remove(p.id, p.name)} disabled={busy || active} title={active ? t('Passe d’abord à un autre utilisateur.') : ''}>{t('Supprimer')}</button>
          {/if}
        </li>
      {/each}
    </ul>
    <div class="row">
      <input type="text" class="grow" bind:value={newName} maxlength={list.max_name} placeholder={t('Nom du nouvel utilisateur')} disabled={busy} aria-label={t('Nom du nouvel utilisateur')}
        onkeydown={(e) => { if (e.key === 'Enter' && newName.trim()) void create(); }} />
      <button class="primary" onclick={create} disabled={busy || !newName.trim()}>{t('Créer un utilisateur')}</button>
    </div>
    <div class="row"><button onclick={openImport} disabled={busy || importOpen}>{t('Importer un utilisateur…')}</button></div>
    <label class="muted"><input type="checkbox" bind:checked={copy} disabled={busy} /> {t('Copier l’utilisateur actif (ses réglages de personne et son dictionnaire, sans son historique ni ses statistiques)')}</label>
  {/if}
</Card>

{#if exportId}
  {#key exportId}<UserExport id={exportId} onclose={() => (exportId = null)} />{/key}
{/if}
{#if importOpen}
  <UserImport onclose={() => (importOpen = false)} onchanged={load} onuse={(id) => { importOpen = false; void use(id); }} />
{/if}

<style>
  .users { list-style: none; margin: 0 0 8px; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  .users li { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; padding: 6px 10px; border: 1px solid var(--line); border-radius: 8px; }
  .users li.active { border-color: var(--accent); }
  .name { flex: 1; min-width: 8em; }
  .grow { flex: 1; min-width: 10em; }
  .warn { color: var(--warn); }
</style>
