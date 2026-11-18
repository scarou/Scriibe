<script lang="ts">
  import PageHeader from '../components/PageHeader.svelte';
  import ActivationCommands from '../components/commands/ActivationCommands.svelte';
  import BuiltinCommands from '../components/commands/BuiltinCommands.svelte';
  import SiteCommands from '../components/commands/SiteCommands.svelte';
  import SnippetCommands from '../components/commands/SnippetCommands.svelte';
  import AppCommands from '../components/commands/AppCommands.svelte';
  import CustomCommands from '../components/commands/CustomCommands.svelte';
  import RewriteCommands from '../components/commands/RewriteCommands.svelte';
  import { settings } from '../lib/settings.svelte';
  import { t } from '../lib/i18n.svelte';
  import { tabDescription } from '../lib/tabs';
</script>

<PageHeader title={t('Commandes vocales')}>{t(tabDescription('commands'))}</PageHeader>

<ActivationCommands />
<!-- Interrupteur coupé : le reste est grisé et ne répond plus (souris : pointer-events ; clavier et lecteurs d'écran : inert). -->
<div class:off={settings.value?.voiceCommandsEnabled === false} inert={settings.value?.voiceCommandsEnabled === false}>
  <BuiltinCommands />
  <SiteCommands />
  <SnippetCommands />
  <AppCommands />
  <CustomCommands />
  <RewriteCommands />
</div>

<style>
  .off { opacity: 0.5; pointer-events: none; }
</style>
