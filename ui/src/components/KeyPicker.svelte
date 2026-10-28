<script lang="ts">
  import { settings } from '../lib/settings.svelte';
  import { status } from '../lib/status.svelte';

  let s = $derived(status.value);
  let v = $derived(settings.value);
  /** Force les menus à reprendre la valeur enregistrée quand un choix est refusé. */
  let version = $state(0);
  let testing = $state(false);
  let detected = $state(false);

  const DEFAULT_KEYS = ['right_control'];
  let current = $derived(v && v.dictationKeys.length ? v.dictationKeys : DEFAULT_KEYS);
  let choices = $derived(s?.key_choices ?? []);
  let modifiers = $derived(choices.filter((c) => c.modifier));
  let others = $derived(choices.filter((c) => !c.modifier));

  async function setKeys(index: number, id: string) {
    const next = [...current];
    if (id) next[index] = id;
    else next.splice(index, 1);
    const unique = next.filter((key, i) => key && next.indexOf(key) === i);
    // Ctrl droit seul est la valeur d'origine : on n'écrit rien dans les réglages.
    const keys = unique.length === 1 && unique[0] === DEFAULT_KEYS[0] ? [] : unique;
    await settings.change({ dictationKeys: keys });
    version++;
  }

  async function setTranslate(id: string) {
    await settings.change({ translateKeys: id ? [id] : [] });
    version++;
  }

  async function reset() {
    await settings.change({ dictationKeys: [], translateKeys: [] });
    version++;
  }

  /** Quinze secondes pendant lesquelles on relit l'état toutes les 0,3 s : la touche maintenue doit faire passer Scriibe en écoute. */
  async function test() {
    testing = true;
    detected = false;
    const end = Date.now() + 15000;
    while (Date.now() < end && !detected) {
      await status.refresh();
      if (status.value?.recording) detected = true;
      else await new Promise((r) => setTimeout(r, 300));
    }
    testing = false;
  }
</script>

{#if s && v}
  {#key version}
    <div class="row">
      <span>Touche de dictée :</span>
      {#each [0, 1, 2] as index (index)}
        {#if index === 0 || current[index - 1]}
          <select value={current[index] ?? ''} onchange={(e) => void setKeys(index, e.currentTarget.value)} aria-label={`Touche ${index + 1} de la dictée`}>
            {#if index > 0}<option value="">{index === 1 ? '+ une autre touche (facultatif)' : '+ une troisième touche (facultatif)'}</option>{/if}
            <optgroup label="Ctrl, Alt, Maj">{#each modifiers as c (c.id)}<option value={c.id}>{c.label}</option>{/each}</optgroup>
            <optgroup label="Autres touches">{#each others as c (c.id)}<option value={c.id}>{c.label}</option>{/each}</optgroup>
          </select>
        {/if}
      {/each}
    </div>
    <div class="row">
      <label>Touche tenue en plus pour traduire :
        <select value={v.translateKeys[0] ?? ''} onchange={(e) => void setTranslate(e.currentTarget.value)}>
          <option value="">Maj (gauche ou droite)</option>
          {#each choices as c (c.id)}<option value={c.id}>{c.label}</option>{/each}
        </select>
      </label>
    </div>
    <div class="row">
      <button onclick={test} disabled={testing}>{testing ? 'Maintiens ta touche…' : 'Tester ma touche'}</button>
      <button onclick={reset} disabled={!v.dictationKeys.length && !v.translateKeys.length}>Rétablir Ctrl droit + Maj</button>
      {#if detected}<span class="ok">✔ Touche reconnue : Scriibe écoute ({s.dictation_label}).</span>{:else if testing}<span class="muted">Maintiens {s.dictation_label}…</span>{/if}
    </div>
    <p class="muted">
      Une combinaison se tient en entier (par exemple Ctrl gauche + F9) : l’une des touches seule ne fait rien. Une touche seule doit être Ctrl droit ou une
      touche qui ne sert pas à taper (F1 à F24, Pause…) : Ctrl gauche, Alt ou Maj seuls se déclencheraient à chaque raccourci (Ctrl+C, Alt+Tab…), qui
      annule alors la dictée. AltGr (nécessaire à « @ » et « € » sur un clavier AZERTY) et la touche Windows ne sont pas proposés. Si Alt gauche fait
      partie de la combinaison, Windows peut activer le menu de l’appli au relâchement : préfère Ctrl et une touche de fonction.
    </p>
  {/key}
{/if}
