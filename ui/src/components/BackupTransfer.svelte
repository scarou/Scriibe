<script lang="ts">
  import Card from './Card.svelte';
  import { dialog } from '../lib/dialog.svelte';
  import { api } from '../lib/api';
  import { t, tx } from '../lib/i18n.svelte';
  import { nav, type TransferKind } from '../lib/nav.svelte';

  /** Une ligne par export et import d'une partie : le bouton ouvre la même carte secondaire que celle de sa page. Aucun format de fichier ici. */
  const rows: { label: string; exp: TransferKind; imp: TransferKind }[] = [
    { label: tx('Utilisateur'), exp: 'user-export', imp: 'user-import' },
    { label: tx('Dictionnaire'), exp: 'dictionary-export', imp: 'dictionary-import' },
    { label: tx('Modes personnels'), exp: 'modes-export', imp: 'modes-import' },
    { label: tx('Commandes vocales personnelles'), exp: 'commands-export', imp: 'commands-import' },
  ];

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
    const ok = await dialog.confirm(t('Remplacer tes réglages, ton dictionnaire et ton historique par ceux de cette sauvegarde ? Une copie de l’état actuel est gardée dans le dossier de données (avant-import-…), puis Scriibe redémarre.'), { confirm: t('Remplacer'), danger: true });
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
</script>

<Card help="data" id="sauvegarde" title={t('Sauvegarder et transférer')} intro={t('Tout sauvegarder d’un coup, ou n’exporter qu’une partie (un utilisateur, le dictionnaire, les modes, les commandes) pour la reprendre sur un autre ordinateur.')}>
  <h3 class="sub">{t('Sauvegarde complète')}</h3>
  <p class="muted">{t('Un seul fichier (.scriibe.json) avec tes réglages et modes, ton dictionnaire, ton historique et tes statistiques. Les clés d’API, les modèles téléchargés, les bancs d’essai et les journaux n’y sont pas.')}</p>
  <p class="muted">{t('L’historique contient les textes que tu as dictés : le fichier reste là où tu le ranges, en clair.')}</p>
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
          <li>{t('tes réglages et {n} mode(s) personnel(s)', { n: p.modes })}</li>
          <li>{t('ton dictionnaire : {words} mot(s), {corrections} correction(s)', { words: p.words, corrections: p.corrections })}</li>
          <li>{t('ton historique : {n} dictée(s) et les statistiques de {days} jour(s)', { n: p.dictations, days: p.days })}</li>
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

  <h3 class="sub">{t('Exporter ou importer une partie')}</h3>
  <p class="muted">{t('Chaque ligne ouvre, sur sa page, la carte d’export ou d’import que tu y trouves aussi.')}</p>
  <ul class="parts">
    {#each rows as r (r.label)}
      <li>
        <span class="name">{t(r.label)}</span>
        <button onclick={() => nav.transfer(r.exp)}>{t('Exporter…')}</button>
        <button onclick={() => nav.transfer(r.imp)}>{t('Importer…')}</button>
      </li>
    {/each}
    <li>
      <span class="name">{t('Conservation de l’historique')}</span>
      <button onclick={() => nav.go('history', 'conservation')}>{t('Régler la conservation →')}</button>
    </li>
  </ul>
  <p class="muted">{t('L’historique et les statistiques ne partent que dans la sauvegarde complète.')}</p>
</Card>

<style>
  .summary { margin: 8px 0; padding: 8px 12px; border: 1px solid var(--line); border-radius: 8px; background: var(--bg); }
  .summary ul { margin: 4px 0 8px 18px; padding: 0; }
  .warn { color: var(--warn); }
  .sub { margin: 12px 0 4px; font-size: .95rem; }
  .sub:first-child { margin-top: 0; }
  .parts { list-style: none; margin: 4px 0 8px; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  .parts li { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; padding: 6px 10px; border: 1px solid var(--line); border-radius: 8px; }
  .parts .name { flex: 1; min-width: 8em; font-weight: 600; }
</style>
