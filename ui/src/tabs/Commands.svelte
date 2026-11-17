<script lang="ts">
  import PageHeader from '../components/PageHeader.svelte';
  import { t } from '../lib/i18n.svelte';
  import { tabDescription } from '../lib/tabs';
  import VoiceCommands from '../components/VoiceCommands.svelte';
</script>

<PageHeader title={t('Commandes vocales')}>{t(tabDescription('commands'))}</PageHeader>

<VoiceCommands />
