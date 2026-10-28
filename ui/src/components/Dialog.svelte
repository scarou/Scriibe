<script lang="ts">
  import { dialog } from '../lib/dialog.svelte';

  let cancel = $state<HTMLButtonElement | undefined>();
  let accept = $state<HTMLButtonElement | undefined>();

  // Le focus va au bouton qui valide (au bouton « Annuler » pour une action définitive : un Entrée distrait ne doit rien effacer).
  $effect(() => {
    if (dialog.current) (dialog.current.danger ? cancel : accept)?.focus();
  });

  function keydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      dialog.answer(false);
    }
  }
</script>

<svelte:window onkeydown={dialog.current ? keydown : undefined} />

{#if dialog.current}
  {@const d = dialog.current}
  <div class="veil" role="presentation">
    <div class="box" role="alertdialog" aria-modal="true" aria-labelledby="dialog-title" aria-describedby="dialog-text">
      <div id="dialog-title" class="title">Scriibe</div>
      <div id="dialog-text" class="text">{d.message}</div>
      <div class="actions">
        {#if d.confirm !== null}
          <button bind:this={cancel} onclick={() => dialog.answer(false)}>Annuler</button>
          <button bind:this={accept} class:danger={d.danger} class:primary={!d.danger} onclick={() => dialog.answer(true)}>{d.confirm}</button>
        {:else}
          <button bind:this={accept} class="primary" onclick={() => dialog.answer(true)}>OK</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .veil { position: fixed; inset: 0; background: rgba(10, 14, 22, 0.45); display: flex; align-items: center; justify-content: center; z-index: 100; padding: 16px; }
  .box { background: var(--card); color: var(--ink); border: 1px solid var(--line); border-radius: 14px; padding: 18px 20px; width: min(460px, 100%); box-shadow: 0 12px 40px rgba(0, 0, 0, 0.3); }
  .title { font-weight: 700; color: var(--brand); margin-bottom: 8px; }
  .text { white-space: pre-wrap; word-break: break-word; max-height: 50vh; overflow: auto; }
  .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 16px; }
  button.danger { background: var(--bad); color: #fff; border-color: var(--bad); }
</style>
