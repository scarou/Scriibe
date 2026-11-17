<script lang="ts">
  import PageHeader from '../components/PageHeader.svelte';
  import { t } from '../lib/i18n.svelte';
  import { tabDescription } from '../lib/tabs';
  import Settings from './Settings.svelte';
  import Users from '../components/Users.svelte';
</script>

<PageHeader title={t('Profil')}>{t(tabDescription('profile'))}</PageHeader>

<Users />
<Settings section="identity" />
<Settings section="keys" />
