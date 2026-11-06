<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type InstalledModel } from '../lib/api';
  import { confirmBox, notice } from '../lib/dialog.svelte';
  import HelpButton from './HelpButton.svelte';
  import { taille } from '../lib/display.svelte';
  import { t, tx } from '../lib/i18n.svelte';
  import { status } from '../lib/status.svelte';

  let folder = $state('');
  let models = $state<InstalledModel[]>([]);

  const kinds = { llama: tx('mise au propre'), whisper: tx('transcription'), partial: tx('téléchargement interrompu') } as const;

  async function load() {
    const out = await api<{ folder: string; models: InstalledModel[] }>('models');
    folder = out.folder;
    models = out.models;
  }

  async function remove(m: InstalledModel) {
    const warn = m.used ? '\n\n' + t('C’est le modèle utilisé en ce moment : il faudra en choisir un autre (Réglages) ou en installer un.') : '';
    if (!(await confirmBox(`${t('Supprimer {name} ({size}) ?', { name: m.name, size: taille(m.size) })}\n${m.path}${warn}`, { confirm: t('Supprimer'), danger: true }))) return;
    const out = await api('models/delete', { path: m.path, name: m.name });
    if (!out.ok) await notice(out.error);
    await load();
    await status.refresh();
  }

  onMount(() => {
    void load();
    const timer = setInterval(() => void load(), 3000);
    return () => clearInterval(timer);
  });
</script>

<section class="card">
  <div class="help-head"><h2>{t('Modèles installés')}</h2><HelpButton section="start" /></div>
  {#if models.length}
    <table>
      <tbody>
        {#each models as m (m.path)}
          <tr>
            <td>{m.name}{#if m.used} <span class="tag">{t('utilisé')}</span>{/if}</td>
            <td class="muted">{t(kinds[m.kind])}</td>
            <td class="muted">{taille(m.size)}</td>
            <td><button onclick={() => remove(m)}>{t('Supprimer')}</button></td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <p class="muted">{t('Aucun modèle dans le dossier de Scriibe pour l’instant.')}</p>
  {/if}
  <p class="muted">{t('Dossier : {folder}. Les modèles d’autres programmes (LM Studio…) sont aussi utilisables, mais ne sont pas listés ici : Scriibe ne les supprime jamais.', { folder })}</p>
</section>
