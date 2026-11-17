<script lang="ts">
  import Card from '../Card.svelte';
  import KeyPicker from '../KeyPicker.svelte';
  import { settings } from '../../lib/settings.svelte';
  import { status } from '../../lib/status.svelte';
  import { keyName, t } from '../../lib/i18n.svelte';

  let v = $derived(settings.value);
  let s = $derived(status.value);
</script>

<Card
  id="commandes-activation"
  title={t('Activer les commandes vocales')}
  help="dictionary"
  intro={t('Quand du texte est sélectionné, une phrase dite seule (rien d’autre dans la dictée) agit sur lui au lieu d’être collée. Sans sélection, ou avec d’autres mots, elle reste du texte. Les phrases affichées sont celles de la langue de l’interface (page Apparence).')}
>
  <p class="warn">{t('Les commandes vocales ne marchent pas encore partout : celles qui agissent sur du texte sélectionné ne fonctionnent que sur Mac pour l’instant.')}</p>
  {#if v}
    <div class="row">
      <label>
        <input type="checkbox" checked={v.voiceCommandsEnabled !== false} onchange={(e) => settings.change({ voiceCommandsEnabled: e.currentTarget.checked })} />
        {t('Activer les commandes vocales')}
      </label>
    </div>
  {/if}
  {#if s?.gnome_shortcut}
    <p class="muted">{s.gnome_command_shortcut ? t('Pour dicter une commande : {shortcut} (même endroit pour le changer).', { shortcut: keyName(s.gnome_command_shortcut) }) : t('Pas encore de raccourci de commande : relance Scriibe, GNOME t’en fera choisir un.')}</p>
  {:else}
    <KeyPicker kinds={['command']} />
  {/if}
</Card>

<style>
  .warn { color: var(--warn); margin: 0 0 8px; }
</style>
