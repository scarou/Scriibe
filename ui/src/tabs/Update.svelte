<script lang="ts">
  import Card from '../components/Card.svelte';
  import { confirmBox, notice } from '../lib/dialog.svelte';
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { percent } from '../lib/format';
  import { mo } from '../lib/display.svelte';
  import { t } from '../lib/i18n.svelte';
  import { settings } from '../lib/settings.svelte';

  interface UpdateState {
    windows: boolean;
    mac: boolean;
    /** Vrai sous Windows et sur Mac : Scriibe sait installer la nouvelle version. */
    installs: boolean;
    current: string;
    commit: string | null;
    repo: string;
    progress: { state: 'idle' | 'running' | 'installed' | 'failed'; done?: number; total?: number; error?: string };
  }
  interface Found {
    ok: boolean;
    error?: string;
    up_to_date: boolean;
    /** Faux sous Linux, ou si la Release n'a pas de fichier pour ce système : Scriibe signale la version mais ne l'installe pas. */
    installable: boolean;
    version: string;
    title: string;
    date: string;
    size: number;
    source: string;
  }

  let u = $state<UpdateState | null>(null);
  let found = $state<Found | null>(null);
  let searching = $state(false);

  async function load() {
    u = await api<UpdateState>('update');
  }

  async function check() {
    searching = true;
    found = await api<Found>('update/check', {});
    searching = false;
  }

  async function install() {
    if (!found) return;
    if (!(await confirmBox(
        [t('Installer la version {version} ({size})', { version: found.version, size: mo(found.size) }), t('depuis {source} ?', { source: found.source }), '', t('Scriibe va redémarrer.')].join('\n'),
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

{#if u}
  <Card id="update-version" title={t('Version de Scriibe')}>
    <div class="muted">
      {t('Version installée : {version}', { version: u.current })}{u.installs ? '' : t(' — sur ce système, Scriibe signale une nouvelle version mais ne l’installe pas encore')}
    </div>

    <div class="row">
      <button onclick={check} disabled={searching}>{t('Chercher une nouvelle version')}</button>
    </div>
    {#if searching}<span class="muted">{t('Recherche…')}</span>{/if}
    {#if found}
      {#if !found.ok}
        <div class="bad">{found.error}</div>
      {:else}
        <div class="muted">
          {t('Dernière version :')} <b>{found.version}</b> — « {found.title} »<br />
          {#if found.installable}
            {t('Publiée le {date} ·', { date: found.date })} <b>{mo(found.size)}</b><br />
          {:else}
            {t('Publiée le {date}', { date: found.date })}<br />
          {/if}
          {t('Source : {source}', { source: found.source })}
        </div>
        {#if found.up_to_date}
          <div class="ok">{t('✔ Vous avez déjà cette version.')}</div>
        {:else if found.installable}
          <div class="row"><button class="primary" onclick={install}>{t('Installer et redémarrer')}</button></div>
        {:else if u.installs}
          <div class="warn">{t('Une version plus récente existe, mais elle ne contient pas encore de fichier pour ce système : réessayez plus tard.')}</div>
        {:else}
          <div class="warn">{t('Une version plus récente existe. Mettez à jour à la main en suivant les étapes ci-dessous.')}</div>
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
  </Card>
  <Card id="update-guide" folded title={t('Comment ça marche ?')}>
    <p>{t('Scriibe est amélioré de temps en temps. Cette page dit si une version plus récente que la tienne existe. Rien n’est jamais changé sans que tu le demandes.')}</p>
    <ul>
      <li><b>{t('Version installée')}</b> — {t('c’est le numéro de la version que tu utilises (par exemple 0.1.0). Il sert à la comparer avec la dernière version publiée.')}</li>
      <li><b>{t('Chercher une nouvelle version')}</b> — {t('Scriibe pose une seule question à GitHub, le site où il est publié : « quelle est la dernière version ? ». Rien de personnel ne part : ni tes dictées, ni tes textes, ni tes réglages.')}</li>
      <li><b>{t('Alerte automatique')}</b> — {t('la case est décochée au départ : Scriibe ne cherche alors que lorsque tu cliques. Cochée, il cherche tout seul au démarrage puis toutes les 6 heures. Quand une version plus récente existe, le bouton « Nouvelle version disponible » apparaît dans la barre latérale, et une ligne en haut du menu de l’icône.')}</li>
      <li><b>{t('Tes données')}</b> — {t('l’historique, le dictionnaire, les modes et les réglages sont rangés à part, dans le dossier de données : une mise à jour n’y touche pas.')}</li>
    </ul>
    <h3>{t('Mettre à jour, pas à pas')}</h3>
    {#if u.installs}
      <ol>
        <li>{t('Clique sur « Chercher une nouvelle version » (en haut de cette page).')}</li>
        <li>{t('Si la page écrit « Vous avez déjà cette version », c’est terminé. Sinon, elle montre la version trouvée, sa date et sa taille.')}</li>
        <li>{t('Clique sur « Installer et redémarrer » et lis la fenêtre de confirmation : elle donne la taille et la source du téléchargement.')}</li>
        <li>{t('Confirme, puis attends : une barre montre le téléchargement. Scriibe remplace son programme et redémarre tout seul ; une nouvelle fenêtre s’ouvre au bout de quelques secondes, tu peux fermer l’ancienne.')}</li>
        <li>{t('Si quelque chose ne va pas, l’ancien programme est gardé à côté du nouveau, et le message d’échec dit pourquoi.')}</li>
        {#if u.mac}
          <li>{t('Sur Mac, les autorisations de macOS (Microphone, Surveillance de l’entrée, Contrôle de l’appareil et accès aux données) restent normalement acquises. Si macOS en redemande une, active-la de nouveau dans Réglages Système → Confidentialité et sécurité, puis relance Scriibe.')}</li>
        {/if}
      </ol>
      {#if u.mac}
        <p class="muted">{t('Première installation sur un Mac, avec toutes les étapes (avertissement de macOS, autorisations, moteurs) : fichier docs/INSTALLATION_MACOS.md du dépôt GitHub.')}</p>
      {/if}
    {:else}
      <p>{t('Sur ce système, Scriibe prévient mais n’installe pas encore à ta place : cherche la dernière version sur https://github.com/scarou/Scriibe/releases.')}</p>
    {/if}
    <p class="muted">{t('Si la recherche échoue, le message dit pourquoi (pas de connexion, GitHub qui limite les demandes…). Le Journal note les vérifications qui échouent ou qui trouvent une version.')}</p>
  </Card>
{/if}
