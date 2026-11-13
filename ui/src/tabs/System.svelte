<script lang="ts">
  import Card from '../components/Card.svelte';
  import { notice, dialog } from '../lib/dialog.svelte';
  import { api } from '../lib/api';
  import { t } from '../lib/i18n.svelte';
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
    const ok = await dialog.confirm(t('Remplacer vos réglages, votre dictionnaire et votre historique par ceux de cette sauvegarde ? Une copie de l’état actuel est gardée dans le dossier de données (sauvegarde-avant-import-…), puis Scriibe redémarre.'), { confirm: t('Remplacer'), danger: true });
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

<style>
  .summary { margin: 8px 0; padding: 8px 12px; border: 1px solid var(--line); border-radius: 8px; background: var(--bg); }
  .summary ul { margin: 4px 0 8px 18px; padding: 0; }
  .warn { color: var(--warn); }
</style>
