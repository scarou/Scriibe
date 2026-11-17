<script lang="ts">
  import Card from '../Card.svelte';
  import { api } from '../../lib/api';
  import { settings } from '../../lib/settings.svelte';
  import { i18n, t } from '../../lib/i18n.svelte';
  import '../commands/commands.css';
  import { takenMessage, wordsIn, type Check, type Site } from './shared';

  let v = $derived(settings.value);
  let drafts = $state<Record<string, string>>({});
  let errors = $state<Record<string, string>>({});
  let sites = $derived((v?.webSearches ?? []) as Site[]);
  let siteDraft = $state({ name: '', url: '', phrase: '' });
  let siteError = $state('');
  let siteHosts = $state<Record<string, string>>({});

  const siteTooShort = () => t('Une phrase de recherche compte au moins deux mots (par exemple « nouvelle recherche amazon »).');
  const checkUrl = (url: string) => api<{ ok: boolean; host?: string; error?: string }>('web-searches/check', { url });
  const nextSiteId = () => 'recherche.' + (Math.max(9, ...sites.map((s) => Number(s.id.split('.')[1]) || 0)) + 1);

  async function addSite() {
    const name = siteDraft.name.trim();
    const url = siteDraft.url.trim();
    const phrase = siteDraft.phrase.trim();
    if (!name || !url || !phrase) {
      siteError = t('Donne un nom, une adresse avec {q} à l’endroit où va le texte cherché, et une phrase à dire.', { q: '{q}' });
      return;
    }
    const address = await checkUrl(url);
    if (!address.ok) {
      siteError = t(address.error ?? 'Adresse refusée');
      return;
    }
    if (wordsIn(phrase) < 2) {
      siteError = siteTooShort();
      return;
    }
    const check = await api<Check>('commands/validate-phrase', { id: 'recherche.nouveau', lang: i18n.lang, phrase });
    if (!check.ok) {
      siteError = check.error === 'taken' ? takenMessage(check.other) : siteTooShort();
      return;
    }
    siteError = '';
    await settings.change({ webSearches: [...sites, { id: nextSiteId(), name, url, phrases: [check.phrase!] }] });
    siteDraft = { name: '', url: '', phrase: '' };
  }

  const updateSite = (id: string, patch: Partial<Site>) => settings.change({ webSearches: sites.map((s) => (s.id === id ? { ...s, ...patch } : s)) });
  const removeSite = (id: string) => settings.change({ webSearches: sites.filter((s) => s.id !== id) });

  async function addSitePhrase(s: Site) {
    const key = s.id + ':phrase';
    const typed = (drafts[key] ?? '').trim();
    if (!typed) return;
    if (wordsIn(typed) < 2) {
      errors[key] = siteTooShort();
      return;
    }
    const check = await api<Check>('commands/validate-phrase', { id: s.id, lang: i18n.lang, phrase: typed });
    if (!check.ok) {
      errors[key] = check.error === 'taken' ? takenMessage(check.other) : siteTooShort();
      return;
    }
    errors[key] = '';
    drafts[key] = '';
    if (!s.phrases.some((p) => p.toLowerCase() === check.phrase!.toLowerCase())) await updateSite(s.id, { phrases: [...s.phrases, check.phrase!] });
  }

  // L'hôte que fixe l'adresse de chaque site, redemandé quand elle change.
  $effect(() => {
    for (const s of sites) void checkUrl(s.url).then((r) => (siteHosts[s.id] = r.host ?? ''));
  });
</script>

<Card id="commandes-sites" title={t('Mes sites de recherche')} help="dictionary">
<div class="vc">
    <p class="muted">{t('Ajoute un site de recherche : dis ta phrase suivie de ce que tu cherches, et l’adresse s’ouvre dans ton navigateur, avec ce texte à la place de {q}. L’adresse doit commencer par https:// et le nom du site doit être fixe : {q} ne peut pas être avant le premier /. Ça ne marche que sous la touche de commande.', { q: '{q}' })}</p>
    {#each sites as s (s.id)}
      <div class="command">
        <div class="head">
          <b>{s.name}</b>
          <span class="muted">{t('Ouvre {host}', { host: siteHosts[s.id] ?? '' })}</span>
          <button class="link" onclick={() => removeSite(s.id)}>{t('Supprimer')}</button>
        </div>
        <div class="langrow">
          <span class="lang muted">{t('Phrases')}</span>
          <div class="chips">
            {#each s.phrases as phrase, i (phrase)}
              <span class="chip">{phrase}<button class="x" title={s.phrases.length < 2 ? t('Une commande garde au moins une phrase') : t('Retirer')} disabled={s.phrases.length < 2} onclick={() => updateSite(s.id, { phrases: s.phrases.filter((_, j) => j !== i) })}>×</button></span>
            {/each}
            <input type="text" class="phrase" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Ajouter une phrase')} bind:value={drafts[s.id + ':phrase']} onkeydown={(e) => e.key === 'Enter' && addSitePhrase(s)} />
            <button onclick={() => addSitePhrase(s)} disabled={!(drafts[s.id + ':phrase'] ?? '').trim()}>{t('Ajouter')}</button>
          </div>
        </div>
        {#if errors[s.id + ':phrase']}<p class="error">{errors[s.id + ':phrase']}</p>{/if}
      </div>
    {/each}
    {#if sites.length === 0}<p class="muted">{t('Aucun site à toi pour l’instant.')}</p>{/if}
    <div class="command newcommand">
      <div class="head"><b>{t('Nouveau site')}</b></div>
      <div class="chips">
        <input type="text" placeholder={t('Nom (ex. : Amazon)')} bind:value={siteDraft.name} />
        <input type="text" class="phrase" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Adresse (ex. : https://www.amazon.fr/s?k={q})', { q: '{q}' })} bind:value={siteDraft.url} />
        <input type="text" class="phrase" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Phrase à dire (ex. : nouvelle recherche amazon)')} bind:value={siteDraft.phrase} />
        <button onclick={addSite}>{t('Ajouter le site')}</button>
      </div>
      {#if siteError}<p class="error nomargin">{siteError}</p>{/if}
    </div>
</div>
</Card>
