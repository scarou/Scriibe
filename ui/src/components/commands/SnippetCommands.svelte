<script lang="ts">
  import Card from '../Card.svelte';
  import { api } from '../../lib/api';
  import { settings } from '../../lib/settings.svelte';
  import { i18n, t } from '../../lib/i18n.svelte';
  import '../commands/commands.css';
  import { isOff, takenMessage, snippetPhraseMessage, type Check, type Snippet } from './shared';
  import CommandSwitch from './CommandSwitch.svelte';

  let v = $derived(settings.value);
  let drafts = $state<Record<string, string>>({});
  let errors = $state<Record<string, string>>({});
  const SNIPPET_MAX = 5000;
  let snippets = $derived((v?.snippets ?? []) as Snippet[]);
  let snippetDraft = $state({ name: '', phrase: '', text: '' });
  let snippetError = $state('');
  let textDrafts = $state<Record<string, string>>({});
  const nextSnippetId = () => 'extrait.' + (Math.max(0, ...snippets.map((s) => Number(s.id.split('.')[1]) || 0)) + 1);
  const checkSnippetText = (text: string) => api<{ ok: boolean; text?: string; error?: string }>('snippets/check', { text });

  async function addSnippet() {
    const name = snippetDraft.name.trim();
    const phrase = snippetDraft.phrase.trim();
    if (!name || !phrase || !snippetDraft.text.trim()) {
      snippetError = t('Donne un nom, une phrase à dire et le texte à coller.');
      return;
    }
    const text = await checkSnippetText(snippetDraft.text);
    if (!text.ok) {
      snippetError = t(text.error ?? 'Texte refusé');
      return;
    }
    const check = await api<Check>('commands/validate-phrase', { id: 'extrait.nouveau', lang: i18n.lang, phrase });
    if (!check.ok) {
      snippetError = check.error === 'taken' ? takenMessage(check.other) : snippetPhraseMessage();
      return;
    }
    snippetError = '';
    await settings.change({ snippets: [...snippets, { id: nextSnippetId(), name, phrases: [check.phrase!], text: text.text! }] });
    snippetDraft = { name: '', phrase: '', text: '' };
  }

  const updateSnippet = (id: string, patch: Partial<Snippet>) => settings.change({ snippets: snippets.map((s) => (s.id === id ? { ...s, ...patch } : s)) });
  const removeSnippet = (id: string) => settings.change({ snippets: snippets.filter((s) => s.id !== id) });

  async function addSnippetPhrase(s: Snippet) {
    const key = s.id + ':phrase';
    const typed = (drafts[key] ?? '').trim();
    if (!typed) return;
    const check = await api<Check>('commands/validate-phrase', { id: s.id, lang: i18n.lang, phrase: typed });
    if (!check.ok) {
      errors[key] = check.error === 'taken' ? takenMessage(check.other) : snippetPhraseMessage();
      return;
    }
    errors[key] = '';
    drafts[key] = '';
    if (!s.phrases.some((p) => p.toLowerCase() === check.phrase!.toLowerCase())) await updateSnippet(s.id, { phrases: [...s.phrases, check.phrase!] });
  }

  async function saveSnippetText(s: Snippet) {
    const draft = textDrafts[s.id];
    if (draft === undefined || draft === s.text) return;
    const check = await checkSnippetText(draft);
    const key = s.id + ':text';
    if (!check.ok) {
      errors[key] = t(check.error ?? 'Texte refusé');
      return;
    }
    errors[key] = '';
    delete textDrafts[s.id];
    await updateSnippet(s.id, { text: check.text! });
  }
</script>

<Card id="commandes-extraits" title={t('Mes extraits')} help="commands">
<div class="vc">
    <p class="muted">{t('Un extrait est un texte que tu colles en disant une phrase : « insérer mon adresse email » colle ton adresse. Il est collé tel quel (plusieurs lignes permises, 5 000 signes au plus), jamais gardé dans l’historique, et le presse-papiers d’avant est remis. Ça ne marche que sous la touche de commande ; dite sans elle, la phrase reste du texte. Un extrait peut contenir une adresse ou un IBAN : il n’est exporté que si tu le coches.')}</p>
    {#each snippets as s (s.id)}
      <div class="command" class:off={isOff(s.id)}>
        <div class="head">
          <b>{s.name}</b>
          <span class="muted">{t('{n} signe(s)', { n: s.text.length })}</span>
          <button class="link" onclick={() => removeSnippet(s.id)}>{t('Supprimer')}</button>
          <CommandSwitch id={s.id} />
        </div>
        <div class="langrow">
          <span class="lang muted">{t('Phrases')}</span>
          <div class="chips">
            {#each s.phrases as phrase, i (phrase)}
              <span class="chip">{phrase}<button class="x" title={s.phrases.length < 2 ? t('Une commande garde au moins une phrase') : t('Retirer')} disabled={s.phrases.length < 2} onclick={() => updateSnippet(s.id, { phrases: s.phrases.filter((_, j) => j !== i) })}>×</button></span>
            {/each}
            <input type="text" class="phrase" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Ajouter une phrase')} bind:value={drafts[s.id + ':phrase']} onkeydown={(e) => e.key === 'Enter' && addSnippetPhrase(s)} />
            <button onclick={() => addSnippetPhrase(s)} disabled={!(drafts[s.id + ':phrase'] ?? '').trim()}>{t('Ajouter')}</button>
          </div>
        </div>
        {#if errors[s.id + ':phrase']}<p class="error">{errors[s.id + ':phrase']}</p>{/if}
        <textarea class="snippettext" rows="3" maxlength={SNIPPET_MAX * 2} spellcheck="false" aria-label={t('Texte à coller')} value={textDrafts[s.id] ?? s.text} oninput={(e) => (textDrafts[s.id] = e.currentTarget.value)} onblur={() => saveSnippetText(s)}></textarea>
        {#if errors[s.id + ':text']}<p class="error">{errors[s.id + ':text']}</p>{/if}
      </div>
    {/each}
    {#if snippets.length === 0}<p class="muted">{t('Aucun extrait pour l’instant.')}</p>{/if}
    <div class="command newcommand">
      <div class="head"><b>{t('Nouvel extrait')}</b></div>
      <div class="chips">
        <input type="text" placeholder={t('Nom (ex. : Adresse email)')} bind:value={snippetDraft.name} />
        <input type="text" class="phrase" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Phrase à dire (ex. : insérer mon adresse email)')} bind:value={snippetDraft.phrase} />
      </div>
      <textarea class="snippettext" rows="3" maxlength={SNIPPET_MAX * 2} spellcheck="false" placeholder={t('Texte à coller')} bind:value={snippetDraft.text}></textarea>
      <div class="chips"><button onclick={addSnippet}>{t('Ajouter l’extrait')}</button></div>
      {#if snippetError}<p class="error nomargin">{snippetError}</p>{/if}
    </div>

</div>
</Card>
