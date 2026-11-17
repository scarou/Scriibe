<script lang="ts">
  import Card from '../Card.svelte';
  import { settings } from '../../lib/settings.svelte';
  import { status } from '../../lib/status.svelte';
  import { t } from '../../lib/i18n.svelte';

  let s = $derived(status.value);
</script>

<Card
  id="commandes-reformulation"
  title={t('Reformulation de la sélection')}
  help="dictionary"
  intro={t('Sous la touche de commande, sélectionne un texte puis dis en quelques mots (25 au plus) ce que tu veux en faire : « cherche les fautes », « rends ça plus amical », « raccourcis ». Le texte reformulé remplace la sélection, et un seul « annuler » rend l’original.')}
>
  {#if s?.rewrite?.available && s.rewrite.needsConsent}
    <div class="row">
      <label><input type="checkbox" checked={s.rewrite.given} onchange={(e) => settings.change({ rewriteConsent: e.currentTarget.checked })} />
        {t('Autoriser l’envoi de la sélection à ce moteur')}</label>
    </div>
    <p class="muted">{t('Pour reformuler une sélection sous la touche de commande. Ce moteur n’est pas sur cet ordinateur : sans cette case, la sélection ne lui est jamais envoyée. L’accord vaut pour ce moteur, ce modèle et cette adresse, et tombe dès que l’un des trois change.')}</p>
  {:else if s?.rewrite?.available}
    <p class="muted">{t('Le moteur de mise au propre est sur cet ordinateur : la sélection n’en sort jamais, aucun accord à donner.')}</p>
  {:else}
    <p class="muted">{t('La reformulation n’est pas disponible avec les réglages actuels : il lui faut un moteur de mise au propre.')}</p>
  {/if}
</Card>
