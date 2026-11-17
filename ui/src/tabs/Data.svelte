<script lang="ts">
  import PageHeader from '../components/PageHeader.svelte';
  import { t } from '../lib/i18n.svelte';
  import { tabDescription } from '../lib/tabs';
  import Settings from './Settings.svelte';
  import BackupTransfer from '../components/BackupTransfer.svelte';
  import System from './System.svelte';
  import Update from './Update.svelte';
</script>

<PageHeader title={t('Données et système')}>{t(tabDescription('data'))}</PageHeader>

<System part="main" />
<BackupTransfer />
<Update />
<Settings section="places" />

<h2 class="part-title">{t('Désinstallation')}</h2>
<System part="uninstall" />
