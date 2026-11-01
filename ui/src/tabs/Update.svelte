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
            {#if found.installable}
              {t('Construite le {date} ·', { date: found.date })} <b>{mo(found.size)}</b><br />
              {t('Source : {source} (exécution n° {run})', { source: found.source, run: found.run })}
            {:else}
              {t('Construite le {date}', { date: found.date })}<br />
              {t('Source : {source}', { source: found.source })}
            {/if}
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
  <section class="card guide">
    <h2>{t('Comment ça marche ?')}</h2>
    <p>{t('Scriibe est amélioré de temps en temps. Cette page dit si une version plus récente que la tienne existe. Rien n’est jamais changé sans que tu le demandes.')}</p>
    <ul>
      <li><b>{t('Version installée')}</b> — {t('c’est le code (sept caractères) de la version que tu utilises. Il sert à la comparer avec la dernière version publiée.')}</li>
      <li><b>{t('Chercher une nouvelle version')}</b> — {t('Scriibe pose une seule question à GitHub, le site où il est publié : « quelle est la dernière version ? ». Rien de personnel ne part : ni tes dictées, ni tes textes, ni tes réglages.')}</li>
      <li><b>{t('Alerte automatique')}</b> — {t('la case est décochée au départ : Scriibe ne cherche alors que lorsque tu cliques. Cochée, il cherche tout seul au démarrage puis toutes les 6 heures. Quand une version plus récente existe, le bouton « Nouvelle version disponible » apparaît dans la barre latérale, et une ligne en haut du menu de l’icône.')}</li>
      <li><b>{t('Le jeton')}</b> — {t('Scriibe n’est pas encore public : GitHub veut savoir qui demande. Le jeton est un mot de passe en lecture seule, qui ne permet que de voir la dernière version. Tu le crées une fois sur github.com (Settings → Developer settings → Fine-grained tokens), pour le dépôt scarou/Scriibe, avec les trois permissions « Actions », « Contents » et « Metadata » en lecture seule (Read-only). Tu le colles ici : Scriibe le range dans le coffre de l’ordinateur et ne l’affiche plus.')}</li>
      <li><b>{t('Tes données')}</b> — {t('l’historique, le dictionnaire, les modes et les réglages sont rangés à part, dans le dossier de données : une mise à jour n’y touche pas.')}</li>
    </ul>
    <h3>{t('Mettre à jour, pas à pas')}</h3>
    {#if u.windows}
      <ol>
        <li>{t('Clique sur « Chercher une nouvelle version » (en haut de cette page). Si le jeton manque, crée-le d’abord comme expliqué plus haut.')}</li>
        <li>{t('Si la page écrit « Vous avez déjà cette version », c’est terminé. Sinon, elle montre la version trouvée, sa date et sa taille.')}</li>
        <li>{t('Clique sur « Installer et redémarrer » et lis la fenêtre de confirmation : elle donne la taille et la source du téléchargement.')}</li>
        <li>{t('Confirme, puis attends : une barre montre le téléchargement. Scriibe remplace son programme et redémarre tout seul ; une nouvelle fenêtre s’ouvre au bout de quelques secondes, tu peux fermer l’ancienne.')}</li>
        <li>{t('Si quelque chose ne va pas, l’ancien programme est gardé à côté du nouveau, et le message d’échec dit pourquoi.')}</li>
      </ol>
    {:else}
      <p>{t('Sur ce système, Scriibe prévient mais n’installe pas encore à ta place. La mise à jour se fait à la main, en cinq étapes :')}</p>
      <ol>
        <li>{t('Quitte Scriibe : clique sur son icône dans la barre des menus (en haut à droite, près de l’horloge), puis sur « Quitter Scriibe ».')}</li>
        <li>{t('Ouvre la page https://github.com/scarou/Scriibe/releases et télécharge le fichier Scriibe-…-macos.zip de la version la plus récente (celle dont le numéro est le plus haut). Il arrive dans le dossier Téléchargements.')}</li>
        <li>{t('Double-clique sur ce fichier dans Téléchargements : l’application Scriibe apparaît à côté.')}</li>
        <li>{t('Fais glisser Scriibe dans le dossier Applications du Finder, puis clique sur « Remplacer ».')}</li>
        <li>{t('Ouvre Scriibe depuis Applications. Tes réglages, ton historique et ton dictionnaire sont conservés. Si macOS redemande une autorisation (Microphone, Surveillance de l’entrée, Contrôle de l’appareil et accès aux données), active-la de nouveau dans Réglages Système → Confidentialité et sécurité, puis relance Scriibe.')}</li>
      </ol>
      <p class="muted">{t('Première installation sur un Mac, avec toutes les étapes (avertissement de macOS, autorisations, moteurs) : fichier docs/INSTALLATION_MACOS.md du dépôt GitHub.')}</p>
    {/if}
    <p class="muted">{t('Si la recherche échoue, le message dit pourquoi (jeton refusé, pas de connexion…). Le Journal note les vérifications qui échouent ou qui trouvent une version, jamais le jeton.')}</p>
  </section>
{/if}
