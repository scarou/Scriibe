<script lang="ts">
  import PageHeader from '../components/PageHeader.svelte';
  import { t } from '../lib/i18n.svelte';
  import { tabDescription } from '../lib/tabs';
  import Bench from './Bench.svelte';
  import Card from '../components/Card.svelte';
  import Journal from './Journal.svelte';
  import TimingsCard from '../components/TimingsCard.svelte';
  import TypingTest from '../components/TypingTest.svelte';
</script>

<PageHeader title={t('Mesures et essais')}>{t(tabDescription('measures'))}</PageHeader>

<Bench />
<TimingsCard />

<Card id="dashboard-vitesse-clavier" folded title={t('Ma vitesse au clavier')} help="measures" intro={t('Sert au calcul du temps gagné. Recopie le texte : seuls les essais complets, à plus de 90 % de lettres justes, comptent.')}>
  <TypingTest />
</Card>

<Journal part="live" />
<Journal part="report" />
