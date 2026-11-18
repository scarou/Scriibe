<script lang="ts">
  import { t } from '../../lib/i18n.svelte';
  import { isOff, setCommandEnabled } from './shared';

  let { id }: { id: string } = $props();
</script>

<label class="inline switch" title={t('Désactiver une commande la retire : sa phrase redevient du texte sans la touche de commande.')}>
  <input type="checkbox" checked={!isOff(id)} onchange={(e) => setCommandEnabled(id, e.currentTarget.checked)} />
  {t('Active')}
</label>
