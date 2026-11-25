<script lang="ts">
  import { about } from '../lib/about';
  import { t } from '../lib/i18n.svelte';
  import { status } from '../lib/status.svelte';
  import { notice } from '../lib/dialog.svelte';

  /** La rubrique « À propos » de l'Aide : nom, version, créateurs, contributeurs, licence, dépôt. L'adresse se copie (aucun lien ne s'ouvre depuis la page). */
  async function copyAddress(address: string) {
    try {
      await navigator.clipboard.writeText(address);
      void notice(t('Adresse copiée.'));
    } catch {
      void notice(t('Copie refusée : sélectionne l’adresse ci-dessous.'));
    }
  }
</script>

<h2>{t('À propos')}</h2>
<p><b>Scriibe</b> — {t('dictée vocale pour Windows, macOS et Linux : tu maintiens une touche, tu parles, tu relâches, et le texte transcrit, mis au propre, se colle dans l’appli ouverte. Scriibe est la suite de Scribe, l’appli de dictée du Mac.')}</p>
<ul>
  <li><b>{t('Version')}</b> : {status.value?.version ?? '…'}</li>
  {#if about.creators.length}<li><b>{t('Créé par')}</b> : {about.creators.join(', ')}</li>{/if}
  {#if about.contributors.length}<li><b>{t('Avec l’aide de')}</b> : {about.contributors.join(', ')}</li>{/if}
  <li><b>{t('Licence')}</b> : MIT. {t('Tu peux utiliser, copier, modifier et partager Scriibe, à condition de garder la mention de droits d’auteur.')}</li>
  <li><b>{t('Code source')}</b> : <code>{about.repository}</code> <button onclick={() => copyAddress(about.repository)}>{t('Copier l’adresse')}</button></li>
  <li><b>{t('Page de soutien')}</b> : <code>{about.supportPage}</code> <button onclick={() => copyAddress(about.supportPage)}>{t('Copier l’adresse')}</button></li>
</ul>
