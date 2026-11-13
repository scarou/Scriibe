<script lang="ts">
  import Card from '../components/Card.svelte';
  import { notice, dialog } from '../lib/dialog.svelte';
  import { api } from '../lib/api';
  import { t, tx } from '../lib/i18n.svelte';
  import { status } from '../lib/status.svelte';
  import { onMount } from 'svelte';

  let osName = $derived(status.os);

  /** Lancer à l'ouverture de session : lu dans le système (rien n'est gardé dans les réglages). */
  type Autostart = { supported: boolean; reason: string | null; enabled: boolean; disabledBySystem: boolean };
  let autostart = $state<Autostart | null>(null);

  async function loadAutostart() {
    autostart = await api<Autostart>('autostart');
  }

  async function setAutostart(on: boolean) {
    const out = await api<Autostart & { ok: boolean; error?: string }>('autostart', { enabled: on });
    if (!out.ok) void notice(out.error ? t(out.error) : t('réglage refusé'));
    await loadAutostart();
  }

  /** Sauvegarde complète : enregistrer, ou restaurer (aperçu, accord, relance). */
  type Preview = { token: string; app: string; exportedAt: string; sameOs: boolean; dictations: number; days: number; words: number; corrections: number; modes: number };
  let saving = $state(false);
  let savedPath = $state('');
  let message = $state('');
  let restoring = $state<{ preview: Preview | null; askPath: boolean; path: string; busy: boolean; message: string } | null>(null);

  async function saveBackup() {
    saving = true;
    message = '';
    savedPath = '';
    const out = await api<{ ok: boolean; saved?: boolean; path?: string; error?: string }>('backup/export', {});
    saving = false;
    if (!out.ok) message = t(out.error ?? 'enregistrement impossible');
    else if (out.saved) savedPath = out.path ?? '';
  }

  async function readBackup(path?: string) {
    if (!restoring) restoring = { preview: null, askPath: false, path: '', busy: true, message: '' };
    restoring.busy = true;
    restoring.message = '';
    const out = await api<{ ok: boolean; cancelled?: boolean; ask_path?: boolean; error?: string } & Partial<Preview>>('backup/import/read', path ? { path } : {});
    if (!restoring) return;
    restoring.busy = false;
    if (out.ok && out.cancelled) restoring = null;
    else if (out.ok && out.token) {
      restoring.askPath = false;
      restoring.preview = out as Preview;
    } else if (out.ask_path) restoring.askPath = true;
    else restoring.message = t(out.error ?? "le fichier n'a pas pu être lu");
  }

  async function applyBackup() {
    if (!restoring?.preview) return;
    const ok = await dialog.confirm(t('Remplacer vos réglages, votre dictionnaire et votre historique par ceux de cette sauvegarde ? Une copie de l’état actuel est gardée dans le dossier de données (avant-import-…), puis Scriibe redémarre.'), { confirm: t('Remplacer'), danger: true });
    if (!ok) return;
    restoring.busy = true;
    const out = await api<{ ok: boolean; error?: string }>('backup/import/apply', { token: restoring.preview.token });
    if (!out.ok) {
      restoring.busy = false;
      restoring.message = t(out.error ?? 'restauration impossible');
    } else restoring.message = t('Restauration préparée : Scriibe redémarre, cette fenêtre va se fermer et se rouvrir.');
  }

  async function cancelRestore() {
    await api('backup/import/cancel', {});
    restoring = null;
  }



  const removalLabels: Record<string, string> = {
    data: tx('Données : réglages, dictionnaire, historique, journaux'),
    engines: tx('Moteurs téléchargés'),
    cache: tx('Cache de la fenêtre'),
    launcher: tx('Lanceur'),
    leftover: tx('Reste de mise à jour'),
    sound: tx('Sons temporaires (effacés directement)'),
    program: tx('Programme'),
  };
  const notBackedUpLabels: Record<string, string> = { models: tx('modèles téléchargés'), benches: tx('bancs d’essai'), logs: tx('journaux'), engines: tx('moteurs téléchargés') };

  /** Désinstaller : la liste exacte vient du serveur (la page ne désigne jamais rien à supprimer). */
  type PlanItem = { kind: string; path: string; size: number; link: boolean; count: number };
  type Plan = {
    ok: boolean;
    blocked: string | null;
    items: PlanItem[];
    kept: { kind: string; path: string; size: number }[];
    notBackedUp: { kind: string; size: number }[];
    warnings: string[];
    startup: boolean;
    keys: 'keychain' | 'credman' | 'env';
    trashSize: number;
    capacityProblem: string | null;
  };
  let removal = $state<{ plan: Plan | null; keepModels: boolean; backup: boolean; busy: boolean; message: string; done: boolean } | null>(null);

  /** « 3,2 Go » : les tailles de la liste. */
  function human(bytes: number): string {
    const units = [tx('octets'), tx('Ko'), tx('Mo'), tx('Go')];
    let value = bytes;
    let unit = 0;
    while (value >= 1000 && unit < units.length - 1) {
      value /= 1000;
      unit++;
    }
    const text = unit === 0 ? String(bytes) : value >= 10 ? String(Math.round(value)) : value.toFixed(1).replace('.', ',');
    return `${text} ${t(units[unit])}`;
  }

  async function loadPlan() {
    if (!removal) return;
    removal.busy = true;
    const plan = await api<Plan>('uninstall/plan', { keepModels: removal.keepModels });
    if (!removal) return;
    removal.plan = plan;
    removal.busy = false;
  }

  async function openRemoval() {
    removal = { plan: null, keepModels: false, backup: true, busy: true, message: '', done: false };
    await loadPlan();
  }

  async function chooseKeepModels(keep: boolean) {
    if (!removal) return;
    removal.keepModels = keep;
    await loadPlan();
  }

  async function removeScriibe() {
    if (!removal?.plan) return;
    const ok = await dialog.confirm(t('Dernière confirmation : Scriibe va se fermer, puis tout ce qui est listé part à la Corbeille. C’est récupérable tant que vous ne la videz pas.'), { confirm: t('Oui, désinstaller'), danger: true });
    if (!ok) return;
    removal.busy = true;
    removal.message = '';
    if (removal.backup) {
      const saved = await api<{ ok: boolean; saved?: boolean; error?: string }>('backup/export', {});
      if (!saved.ok || !saved.saved) {
        removal.busy = false;
        removal.message = saved.ok ? t('Sauvegarde annulée : rien n’est supprimé.') : t(saved.error ?? 'enregistrement impossible') + ' ' + t('Rien n’est supprimé.');
        return;
      }
    }
    const out = await api<{ ok: boolean; error?: string }>('uninstall/run', { keepModels: removal.keepModels, backup: removal.backup, confirmed: true });
    if (!out.ok) {
      removal.busy = false;
      removal.message = t(out.error ?? 'désinstallation impossible') + ' ' + t('Rien n’est supprimé.');
    } else {
      removal.done = true;
      removal.message = t('Scriibe se ferme. Un message dira ce qui est parti et ce qui reste.');
    }
  }

  async function closeRemoval() {
    removal = null;
  }

  onMount(() => void loadAutostart());
</script>

<Card help="start" id="demarrage" title={t('Lancer au démarrage')} intro={t('Scriibe se lance à l’ouverture de votre session, sans fenêtre : l’icône est là et la touche de dictée est prête. Désactivé tant que vous ne le cochez pas.')}>
  {#if autostart}
    <div class="row">
      <label><input type="checkbox" checked={autostart.enabled} disabled={!autostart.supported} onchange={(e) => setAutostart(e.currentTarget.checked)} />
        {t('Lancer Scriibe à l’ouverture de session')}</label>
    </div>
    {#if !autostart.supported}
      <p class="muted">{autostart.reason ? t(autostart.reason) : ''}</p>
    {:else if autostart.disabledBySystem}
      <p class="muted">{t('Cet élément est désactivé dans les réglages du système : cochez la case pour le réactiver.')}</p>
    {/if}
    <p class="muted">{osName === 'macos' ? t('Visible dans Réglages Système → Général → Ouverture.') : osName === 'linux' ? t('Visible dans « Applications au démarrage » de votre environnement de bureau.') : t('Visible dans le Gestionnaire des tâches, onglet « Applications de démarrage ».')}</p>
  {/if}
</Card>

<Card help="start" id="sauvegarde" title={t('Sauvegarde de vos données')} intro={t('Un seul fichier (.scriibe.json) avec vos réglages et modes, votre dictionnaire, votre historique et vos statistiques. Les clés d’API, les modèles téléchargés, les bancs d’essai et les journaux n’y sont pas.')}>
  <p class="muted">{t('L’historique contient les textes que vous avez dictés : le fichier reste là où vous le rangez, en clair.')}</p>
  <div class="row">
    <button onclick={saveBackup} disabled={saving}>{saving ? t('Enregistrement…') : t('Enregistrer une sauvegarde…')}</button>
    <button onclick={() => readBackup()} disabled={restoring?.busy}>{t('Restaurer une sauvegarde…')}</button>
  </div>
  {#if savedPath}<p class="ok">{t('✔ Sauvegarde enregistrée et relue : {path}', { path: savedPath })}</p>{/if}
  {#if message}<p class="warn">{message}</p>{/if}

  {#if restoring}
    <div class="summary">
      {#if restoring.askPath}
        <label>{t('Chemin du fichier de sauvegarde :')}
          <input type="text" class="grow" bind:value={restoring.path} placeholder="~/Documents/scriibe-sauvegarde-….scriibe.json" />
        </label>
        <button onclick={() => readBackup(restoring?.path)} disabled={!restoring.path || restoring.busy}>{t('Lire')}</button>
      {/if}
      {#if restoring.preview}
        {@const p = restoring.preview}
        <p><b>{t('Cette sauvegarde remplacera :')}</b></p>
        <ul>
          <li>{t('vos réglages et {n} mode(s) personnel(s)', { n: p.modes })}</li>
          <li>{t('votre dictionnaire : {words} mot(s), {corrections} correction(s)', { words: p.words, corrections: p.corrections })}</li>
          <li>{t('votre historique : {n} dictée(s) et les statistiques de {days} jour(s)', { n: p.dictations, days: p.days })}</li>
        </ul>
        {#if !p.sameOs}<p class="muted">{t('Elle vient d’un autre système : les touches, le micro, les sons et les chemins de modèles de cet ordinateur sont gardés.')}</p>{/if}
        <p class="muted">{p.app} · {p.exportedAt}</p>
        <div class="row">
          <button class="danger" onclick={applyBackup} disabled={restoring.busy}>{t('Restaurer')}</button>
          <button onclick={cancelRestore} disabled={restoring.busy}>{t('Annuler')}</button>
        </div>
      {:else if !restoring.askPath}
        <p class="muted">{t('Lecture…')}</p>
      {/if}
      {#if restoring.message}<p class="warn">{restoring.message}</p>{/if}
    </div>
  {/if}
</Card>

<Card help="start" id="desinstaller" title={t('Désinstaller Scriibe')} intro={t('Retire Scriibe de cet ordinateur : tout part à la Corbeille (récupérable tant que vous ne la videz pas), après une sauvegarde proposée et une dernière confirmation.')}>
  {#if !removal}
    <button class="danger" onclick={openRemoval}>{t('Désinstaller Scriibe…')}</button>
  {:else if !removal.plan}
    <p class="muted">{t('Lecture…')}</p>
  {:else}
    {@const plan = removal.plan}
    <div class="summary">
      {#if plan.blocked}
        <p class="warn">{t(plan.blocked)}</p>
      {:else}
        <p><b>{t('Part à la Corbeille ({size}) :', { size: human(plan.trashSize) })}</b></p>
        <ul>
          {#each plan.items as it (it.path)}
            <li><b>{t(removalLabels[it.kind] ?? it.kind)}</b> — <code>{it.path}</code> ({human(it.size)}){#if it.count > 1} · {t('{n} fichiers', { n: it.count })}{/if}{#if it.link} · {t('lien : sa cible reste en place')}{/if}</li>
          {/each}
          {#if plan.startup}<li>{t('L’entrée de démarrage est retirée.')}</li>{/if}
          <li>{plan.keys === 'keychain' ? t('Clés d’API effacées du trousseau (deepgram, openrouter, et l’ancien compte github s’il existe). macOS peut redemander l’accès au trousseau, jusqu’à une fois par clé.') : plan.keys === 'credman' ? t('Clés d’API effacées du Gestionnaire d’identifiants (deepgram, openrouter, ancien github).') : t('Linux : Scriibe ne range pas les clés. Retirez DEEPGRAM_API_KEY et OPENROUTER_API_KEY de votre profil de shell si vous les y aviez mises.')}</li>
          {#if osName === 'macos'}<li>{t('Les autorisations de Scriibe (micro, accessibilité, surveillance de l’entrée) sont retirées ; une réinstallation les redemandera.')}</li>{/if}
        </ul>
        {#if plan.kept.length}
          <p><b>{t('Reste en place :')}</b></p>
          <ul>
            {#each plan.kept as k (k.path)}
              <li>{k.kind === 'models' ? t('Vos modèles téléchargés') : t('Modèle hors de Scriibe')} — <code>{k.path}</code>{#if k.size} ({human(k.size)}){/if}</li>
            {/each}
          </ul>
        {/if}
        {#if plan.notBackedUp.length}
          <p class="muted">{t('Non sauvegardé :')} {plan.notBackedUp.map((n) => `${t(notBackedUpLabels[n.kind] ?? n.kind)} (${human(n.size)})`).join(', ')}, {t('clés d’API')}.</p>
        {/if}
        {#each plan.warnings as w (w)}<p class="warn">{t(w)}</p>{/each}
        {#if plan.capacityProblem}<p class="warn">{t(plan.capacityProblem)}</p>{/if}
        <div class="row"><label><input type="checkbox" checked={removal.keepModels} disabled={removal.busy || removal.done} onchange={(e) => chooseKeepModels(e.currentTarget.checked)} />
          {t('Garder mes modèles téléchargés')}</label></div>
        <div class="row"><label><input type="checkbox" bind:checked={removal.backup} disabled={removal.busy || removal.done} />
          {t('Faire d’abord une sauvegarde (recommandé)')}</label></div>
        <p class="muted">{t('Si la sauvegarde échoue ou est annulée, rien n’est supprimé.')}</p>
      {/if}
      <div class="row">
        <button class="danger" onclick={removeScriibe} disabled={!!plan.blocked || !!plan.capacityProblem || plan.items.length === 0 || removal.busy || removal.done}>{t('Désinstaller')}</button>
        <button onclick={closeRemoval} disabled={removal.busy || removal.done}>{t('Annuler')}</button>
      </div>
      {#if removal.message}<p class={removal.done ? 'ok' : 'warn'}>{removal.message}</p>{/if}
    </div>
  {/if}
</Card>

<style>
  .summary { margin: 8px 0; padding: 8px 12px; border: 1px solid var(--line); border-radius: 8px; background: var(--bg); }
  .summary ul { margin: 4px 0 8px 18px; padding: 0; }
  .warn { color: var(--warn); }
</style>
