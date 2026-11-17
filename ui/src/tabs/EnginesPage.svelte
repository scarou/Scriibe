<script lang="ts">
  import PageHeader from '../components/PageHeader.svelte';
  import { t } from '../lib/i18n.svelte';
  import { tabDescription } from '../lib/tabs';
  import Install from './Install.svelte';
  import Settings from './Settings.svelte';
</script>

<PageHeader title={t('Moteurs et modèles')}>{t(tabDescription('engines'))}</PageHeader>

<Settings section="engines" />
<Install />
