<script lang="ts">
  import Card from '../components/Card.svelte';
  import { notice, dialog } from '../lib/dialog.svelte';
  import { api } from '../lib/api';
  import { t, tx } from '../lib/i18n.svelte';
  import { status } from '../lib/status.svelte';
  import { onMount } from 'svelte';

  /** `main` : lancer au démarrage ; `uninstall` : la désinstallation, que la page Données et système met en dernier, à part. */
  let { part }: { part: 'main' | 'uninstall' } = $props();

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
    const ok = await dialog.confirm(t('Dernière confirmation : Scriibe va se fermer, puis tout ce qui est listé part à la Corbeille. C’est récupérable tant que tu ne la vides pas.'), { confirm: t('Oui, désinstaller'), danger: true });
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

{#if part === 'main'}
<Card help="start" id="demarrage" title={t('Lancer au démarrage')} intro={t('Scriibe se lance à l’ouverture de ta session, sans fenêtre : l’icône est là et la touche de dictée est prête. Désactivé tant que tu ne le coches pas.')}>
  {#if autostart}
    <div class="row">
      <label><input type="checkbox" checked={autostart.enabled} disabled={!autostart.supported} onchange={(e) => setAutostart(e.currentTarget.checked)} />
        {t('Lancer Scriibe à l’ouverture de session')}</label>
    </div>
    {#if !autostart.supported}
      <p class="muted">{autostart.reason ? t(autostart.reason) : ''}</p>
    {:else if autostart.disabledBySystem}
      <p class="muted">{t('Cet élément est désactivé dans les réglages du système : coche la case pour le réactiver.')}</p>
    {/if}
    <p class="muted">{osName === 'macos' ? t('Visible dans Réglages Système → Général → Ouverture.') : osName === 'linux' ? t('Visible dans « Applications au démarrage » de ton environnement de bureau.') : t('Visible dans le Gestionnaire des tâches, onglet « Applications de démarrage ».')}</p>
  {/if}
</Card>

{:else}
<Card help="start" id="desinstaller" title={t('Désinstaller Scriibe')} intro={t('Retire Scriibe de cet ordinateur : tout part à la Corbeille (récupérable tant que tu ne la vides pas), après une sauvegarde proposée et une dernière confirmation.')}>
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
          <li>{plan.keys === 'keychain' ? t('Clés d’API effacées du trousseau (deepgram, openrouter, et l’ancien compte github s’il existe). macOS peut redemander l’accès au trousseau, jusqu’à une fois par clé.') : plan.keys === 'credman' ? t('Clés d’API effacées du Gestionnaire d’identifiants (deepgram, openrouter, ancien github).') : t('Linux : Scriibe ne range pas les clés. Retire DEEPGRAM_API_KEY et OPENROUTER_API_KEY de ton profil de shell si tu les y avais mises.')}</li>
          {#if osName === 'macos'}<li>{t('Les autorisations de Scriibe (micro, accessibilité, surveillance de l’entrée) sont retirées ; une réinstallation les redemandera.')}</li>{/if}
        </ul>
        {#if plan.kept.length}
          <p><b>{t('Reste en place :')}</b></p>
          <ul>
            {#each plan.kept as k (k.path)}
              <li>{k.kind === 'models' ? t('Tes modèles téléchargés') : t('Modèle hors de Scriibe')} — <code>{k.path}</code>{#if k.size} ({human(k.size)}){/if}</li>
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
{/if}

<style>
  .summary { margin: 8px 0; padding: 8px 12px; border: 1px solid var(--line); border-radius: 8px; background: var(--bg); }
  .summary ul { margin: 4px 0 8px 18px; padding: 0; }
  .warn { color: var(--warn); }
</style>
