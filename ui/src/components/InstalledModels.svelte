<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type InstalledModel } from '../lib/api';
  import { confirmBox, notice } from '../lib/dialog.svelte';
  import { taille } from '../lib/format';
  import { status } from '../lib/status.svelte';

  let folder = $state('');
  let models = $state<InstalledModel[]>([]);

  const kinds = { llama: 'mise au propre', whisper: 'transcription', partial: 'téléchargement interrompu' } as const;

  async function load() {
    const out = await api<{ folder: string; models: InstalledModel[] }>('models');
    folder = out.folder;
    models = out.models;
  }

  async function remove(m: InstalledModel) {
    const warn = m.used ? '\n\nC’est le modèle utilisé en ce moment : il faudra en choisir un autre (Réglages) ou en installer un.' : '';
    if (!(await confirmBox(`Supprimer ${m.name} (${taille(m.size)}) ?\n${m.path}${warn}`, { confirm: 'Supprimer', danger: true }))) return;
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
  <h2>Modèles installés</h2>
  {#if models.length}
    <table>
      <tbody>
        {#each models as m (m.path)}
          <tr>
            <td>{m.name}{#if m.used} <span class="tag">utilisé</span>{/if}</td>
            <td class="muted">{kinds[m.kind]}</td>
            <td class="muted">{taille(m.size)}</td>
            <td><button onclick={() => remove(m)}>Supprimer</button></td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <p class="muted">Aucun modèle dans le dossier de Scriibe pour l’instant.</p>
  {/if}
  <p class="muted">Dossier : {folder}. Les modèles d’autres programmes (LM Studio…) sont aussi utilisables, mais ne sont pas listés ici : Scriibe ne les supprime jamais.</p>
</section>
