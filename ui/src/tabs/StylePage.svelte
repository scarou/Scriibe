<script lang="ts">
  import PageHeader from '../components/PageHeader.svelte';
  import { t } from '../lib/i18n.svelte';
  import { tabDescription } from '../lib/tabs';
  import Modes from './Modes.svelte';
  import Settings from './Settings.svelte';
</script>

<PageHeader title={t('Style d’écriture')}>{t(tabDescription('style'))}</PageHeader>

<Settings section="style" />
<Modes />
