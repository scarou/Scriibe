<script lang="ts">
  import PageHeader from '../components/PageHeader.svelte';
  import { t } from '../lib/i18n.svelte';
  import { tabDescription } from '../lib/tabs';
  import Settings from './Settings.svelte';
</script>

<PageHeader title={t('Dicter')}>{t(tabDescription('dictate'))}</PageHeader>

<Settings section="dictation" />
