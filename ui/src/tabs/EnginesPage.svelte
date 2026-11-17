<script lang="ts">
  import PageHeader from '../components/PageHeader.svelte';
  import { t } from '../lib/i18n.svelte';
  import { tabDescription } from '../lib/tabs';
  import Install from './Install.svelte';
  import Settings from './Settings.svelte';
</script>

<PageHeader title={t('Moteurs et modèles')}>{t(tabDescription('engines'))}</PageHeader>

<h2 class="part-title">{t('Moteurs')}</h2>
<Settings section="engines" />

<h2 class="part-title">{t('Modèles')}</h2>
<Install />
