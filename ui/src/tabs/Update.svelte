<script lang="ts">
  import { confirmBox, notice } from '../lib/dialog.svelte';
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { percent } from '../lib/format';
  import { mo } from '../lib/display.svelte';
  import { t } from '../lib/i18n.svelte';
  import { settings } from '../lib/settings.svelte';

  interface UpdateState {
    windows: boolean;
    has_token: boolean;
    current: string | null;
    repo: string;
    progress: { state: 'idle' | 'running' | 'installed' | 'failed'; done?: number; total?: number; error?: string };
  }
  interface Found {
    ok: boolean;
    error?: string;
    up_to_date: boolean;
    /** Faux sur Mac et Linux : Scriibe signale la version mais ne l'installe pas. */
    installable: boolean;
    sha: string;
    title: string;
    date: string;
    size: number;
    run: number;
    source: string;
  }

  let u = $state<UpdateState | null>(null);
  let token = $state('');
  let found = $state<Found | null>(null);
  let searching = $state(false);
  let changing = $state(false);

  async function load() {
    u = await api<UpdateState>('update');
  }

  async function saveToken() {
    const out = await api('update/token', { token });
    token = '';
    if (!out.ok) await notice(out.error);
    else changing = false;
    found = null;
    await load();
  }

  async function check() {
    searching = true;
    found = await api<Found>('update/check', {});
    searching = false;
  }

  async function install() {
    if (!found) return;
    if (!(await confirmBox(
        [t('Installer la version {sha} ({size})', { sha: found.sha, size: mo(found.size) }), t('depuis {source} ?', { source: found.source }), '', t('Scriibe va redémarrer.')].join('\n'),
        { confirm: t('Installer et redémarrer') },
      ))) return;
    const out = await api('update/install', { confirmed: true });
    if (!out.ok) await notice(out.error);
    found = null;
    await load();
  }

  onMount(() => {
    void load();
    void settings.load();
    const timer = setInterval(() => void load(), 1500);
    return () => clearInterval(timer);
  });
</script>

<h1>{t('Mise à jour de Scriibe')}</h1>

{#if u}
  <section class="card">
    <div class="muted">
      {t('Version installée : {version}', { version: u.current ?? t('compilée à la main (inconnue)') })}{u.windows ? '' : t(' — sur ce système, Scriibe signale une nouvelle version mais ne l’installe pas encore')}
    </div>

    {#if !u.has_token || changing}
      <p class="muted">
        {t('Pour savoir si une nouvelle version existe sur GitHub (dépôt privé), Scriibe a besoin d’un jeton en lecture seule. Il est gardé dans le gestionnaire d’identifiants du système (Windows) ou le trousseau (Mac), jamais affiché ni écrit dans le journal.')}
      </p>
      <div class="row">
        <input type="password" class="grow" placeholder={t('Colle le jeton GitHub ici')} autocomplete="off" bind:value={token} />
        <button onclick={saveToken} disabled={!token}>{t('Enregistrer le jeton')}</button>
        {#if changing}<button onclick={() => { changing = false; token = ''; }}>{t('Annuler')}</button>{/if}
      </div>
    {/if}

    {#if u.has_token}
      <div class="row">
        <button onclick={check} disabled={searching}>{t('Chercher une nouvelle version')}</button>
        {#if !changing}<button onclick={() => (changing = true)}>{t('Changer le jeton')}</button>{/if}
      </div>
      {#if searching}<span class="muted">{t('Recherche…')}</span>{/if}
      {#if found}
        {#if !found.ok}
          <div class="bad">{found.error}</div>
        {:else}
          <div class="muted">
            {t('Dernière version :')} <b>{found.sha}</b> — « {found.title} »<br />
            {t('Construite le {date} ·', { date: found.date })} <b>{mo(found.size)}</b><br />
            {t('Source : {source} (exécution n° {run})', { source: found.source, run: found.run })}
          </div>
          {#if found.up_to_date}
            <div class="ok">{t('✔ Vous avez déjà cette version.')}</div>
          {:else if found.installable}
            <div class="row"><button class="primary" onclick={install}>{t('Installer et redémarrer')}</button></div>
          {:else}
            <div class="warn">{t('Une version plus récente existe. Pour l’instant, mettez à jour à la main : récupérez le code (git pull), puis recompilez Scriibe.')}</div>
          {/if}
        {/if}
      {/if}
    {/if}

    {#if settings.value}
      <div class="row">
        <label><input type="checkbox" checked={settings.value.checkUpdates} onchange={(e) => settings.change({ checkUpdates: e.currentTarget.checked })} />
          {t('Chercher automatiquement une nouvelle version (toutes les 6 heures)')}</label>
      </div>
    {/if}

    {#if u.progress.state === 'running'}
      <div class="bar"><div style="width: {percent(u.progress.done ?? 0, u.progress.total ?? 0)}%"></div></div>
      <span class="muted">{t('Téléchargement {done} / {total} ({percent} %)', { done: mo(u.progress.done ?? 0), total: mo(u.progress.total ?? 0), percent: percent(u.progress.done ?? 0, u.progress.total ?? 0) })}</span>
    {:else if u.progress.state === 'installed'}
      <div class="ok">{t('✔ Nouvelle version installée. Scriibe redémarre : une nouvelle fenêtre s’ouvre dans quelques secondes (celle-ci peut être fermée).')}</div>
    {:else if u.progress.state === 'failed'}
      <div class="bad">{t('Échec : {error}', { error: u.progress.error ?? '' })}</div>
    {/if}
  </section>
{/if}
