<script lang="ts">
  import Card from '../Card.svelte';
  import { status } from '../../lib/status.svelte';
  import { exportWhere } from '../../lib/exportwhere';
  import { api } from '../../lib/api';
  import { settings } from '../../lib/settings.svelte';
  import { nav } from '../../lib/nav.svelte';
  import { i18n, t } from '../../lib/i18n.svelte';
  import '../commands/commands.css';
  import ShortcutField from '../ShortcutField.svelte';
  import { takenMessage, tooShortMessage, badShortcutMessage, shortcutLabel, type Check, type Custom } from './shared';

  let v = $derived(settings.value);
  let drafts = $state<Record<string, string>>({});
  let errors = $state<Record<string, string>>({});
  let customs = $derived((v?.customCommands ?? []) as Custom[]);
  let customDraft = $state({ name: '', phrase: '', shortcut: '', needsSelection: true });
  let customError = $state('');
  let customLabels = $state<Record<string, string>>({});
  let customShortcutDrafts = $state<Record<string, string>>({});



  async function addCustom() {
    const name = customDraft.name.trim();
    const phrase = customDraft.phrase.trim();
    const shortcut = customDraft.shortcut.trim();
    if (!name || !phrase || !shortcut) {
      customError = t('Donne un titre, une phrase à dire et un raccourci.');
      return;
    }
    const check = await api<Check>('commands/validate-phrase', { id: 'perso.nouveau', lang: i18n.lang, phrase });
    if (!check.ok) {
      customError = check.error === 'taken' ? takenMessage(check.other) : tooShortMessage();
      return;
    }
    if (!(await shortcutLabel(shortcut)).ok) {
      customError = badShortcutMessage();
      return;
    }
    customError = '';
    const entry: Custom = { id: 'perso.' + Date.now(), name, phrases: [check.phrase!], shortcut, needsSelection: customDraft.needsSelection };
    await settings.change({ customCommands: [...customs, entry] });
    customDraft = { name: '', phrase: '', shortcut: '', needsSelection: true };
  }

  /** Exporter et importer les commandes perso : aperçu avant tout, l'import ajoute sans jamais remplacer. */
  type Imported = { name: string; phrases: string[]; dropped: string[]; shortcut: string; needs_selection: boolean; problem: 'no_name' | 'bad_shortcut' | 'no_phrase' | null };
  let transfer = $state<{ mode: 'export' | 'import'; items: Imported[]; tick: boolean[]; token: string; askPath: boolean; path: string; busy: boolean; message: string; done: boolean } | null>(null);

  const problemText = (p: Imported['problem']) =>
    p === 'bad_shortcut' ? t('raccourci illisible : cette commande ne sera pas importée') : p === 'no_phrase' ? t('plus aucune phrase utilisable : cette commande ne sera pas importée') : t('sans titre : cette commande ne sera pas importée');

  async function openTransfer(mode: 'export' | 'import') {
    if (transfer && !transfer.done && transfer.mode === 'import') await api('commands/import/cancel', {});
    transfer = { mode, items: [], tick: [], token: '', askPath: false, path: '', busy: false, message: '', done: false };
    if (mode === 'import') await readImport();
  }

  async function exportCommands() {
    if (!transfer) return;
    transfer.busy = true;
    const out = await api<{ ok: boolean; saved?: boolean; path?: string; error?: string }>('commands/export', {});
    transfer.busy = false;
    transfer.message = !out.ok
      ? t(out.error ?? 'enregistrement impossible : l’emplacement choisi n’est pas accessible en écriture')
      : out.saved
        ? t('Enregistré : {path}', { path: out.path ?? '' })
        : t('Export annulé : rien n’a été enregistré.');
  }

  async function readImport(path?: string) {
    if (!transfer) return;
    transfer.busy = true;
    transfer.message = '';
    const out = await api<{ ok: boolean; cancelled?: boolean; ask_path?: boolean; error?: string; token?: string; items?: Imported[] }>('commands/import/read', path ? { path } : {});
    if (!transfer) return;
    transfer.busy = false;
    if (out.ok && out.cancelled) transfer = null;
    else if (out.ok && out.items) {
      transfer.askPath = false;
      transfer.items = out.items;
      transfer.token = out.token ?? '';
      transfer.tick = out.items.map((i) => i.problem === null);
    } else if (out.ask_path) transfer.askPath = true;
    else transfer.message = t(out.error ?? "le fichier n'a pas pu être lu");
  }

  async function applyImport() {
    if (!transfer) return;
    transfer.busy = true;
    const out = await api<{ ok: boolean; added?: string[]; skipped?: number; error?: string }>('commands/import/apply', { token: transfer.token, include: transfer.tick });
    transfer.busy = false;
    if (!out.ok) {
      transfer.message = t(out.error ?? "le fichier n'a pas pu être lu");
      return;
    }
    transfer.items = [];
    transfer.done = true;
    transfer.message = t('Importé : {n} commande(s) ajoutée(s).', { n: out.added?.length ?? 0 });
    // Le serveur a écrit les réglages : la page les relit, pour ne pas les écraser plus tard.
    await settings.load();
  }

  async function closeTransfer() {
    if (transfer && !transfer.done && transfer.mode === 'import') await api('commands/import/cancel', {});
    transfer = null;
  }

  async function updateCustom(id: string, patch: Partial<Custom>) {
    await settings.change({ customCommands: customs.map((c) => (c.id === id ? { ...c, ...patch } : c)) });
  }

  async function removeCustom(id: string) {
    await settings.change({ customCommands: customs.filter((c) => c.id !== id) });
  }

  async function addCustomPhrase(c: Custom) {
    const key = c.id + ':phrase';
    const typed = (drafts[key] ?? '').trim();
    if (!typed) return;
    const check = await api<Check>('commands/validate-phrase', { id: c.id, lang: i18n.lang, phrase: typed });
    if (!check.ok) {
      errors[key] = check.error === 'taken' ? takenMessage(check.other) : tooShortMessage();
      return;
    }
    errors[key] = '';
    drafts[key] = '';
    if (!c.phrases.some((p) => p.toLowerCase() === check.phrase!.toLowerCase())) await updateCustom(c.id, { phrases: [...c.phrases, check.phrase!] });
  }

  async function setCustomShortcut(c: Custom, text: string) {
    const check = await shortcutLabel(text);
    if (!check.ok) {
      errors[c.id + ':shortcut'] = badShortcutMessage();
      return;
    }
    errors[c.id + ':shortcut'] = '';
    await updateCustom(c.id, { shortcut: text.trim() });
  }

  // Le raccourci lu (« ⌘⇧X ») de chaque commande, redemandé quand le raccourci change.
  $effect(() => {
    for (const c of customs) {
      customShortcutDrafts[c.id] ??= c.shortcut;
      void shortcutLabel(c.shortcut).then((r) => (customLabels[c.id] = r.label ?? ''));
    }
  });

  /** Demande de la carte « Sauvegarder et transférer » : la même carte d'export ou d'import que les boutons de la carte. */
  $effect(() => {
    const kind = nav.transferRequest;
    if (kind !== 'commands-export' && kind !== 'commands-import') return;
    if (!v) return;
    nav.transferRequest = null;
    if (kind === 'commands-import') void openTransfer('import');
    else if (customs.length > 0) void openTransfer('export');
    else nav.transferNothing(t('Aucune commande vocale personnelle à exporter.'));
  });
</script>

<Card id="commandes-perso" title={t('Mes commandes')} help="commands">
<div class="vc">
    <p class="muted">{t('Donne un titre à une phrase qui appuie un raccourci de l’appli active, par exemple ⌘⇧X sur Mac. Clique dans le champ du raccourci, puis appuie sur la combinaison voulue. Un raccourci qui existe déjà dans une appli s’appelle ainsi par la voix.')}</p>
    {#each customs as c (c.id)}
      <div class="command">
        <div class="head">
          <b>{c.name}</b>
          <label class="inline">
            <input type="checkbox" checked={c.needsSelection} onchange={(e) => updateCustom(c.id, { needsSelection: e.currentTarget.checked })} />
            {t('Seulement si du texte est sélectionné')}
          </label>
          <button class="link" onclick={() => removeCustom(c.id)}>{t('Supprimer')}</button>
        </div>
        <div class="langrow">
          <span class="lang muted">{t('Phrases')}</span>
          <div class="chips">
            {#each c.phrases as phrase, i (phrase)}
              <span class="chip">{phrase}<button class="x" title={c.phrases.length < 2 ? t('Une commande garde au moins une phrase') : t('Retirer')} disabled={c.phrases.length < 2} onclick={() => updateCustom(c.id, { phrases: c.phrases.filter((_, j) => j !== i) })}>×</button></span>
            {/each}
            <input type="text" class="phrase" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Ajouter une phrase')} bind:value={drafts[c.id + ':phrase']} onkeydown={(e) => e.key === 'Enter' && addCustomPhrase(c)} />
            <button onclick={() => addCustomPhrase(c)} disabled={!(drafts[c.id + ':phrase'] ?? '').trim()}>{t('Ajouter')}</button>
          </div>
        </div>
        {#if errors[c.id + ':phrase']}<p class="error">{errors[c.id + ':phrase']}</p>{/if}
        <div class="langrow">
          <span class="lang muted">{t('Raccourci')}</span>
          <div class="chips">
            <ShortcutField value={customShortcutDrafts[c.id] ?? c.shortcut} onset={(s) => { customShortcutDrafts[c.id] = s; void setCustomShortcut(c, s); }} />
            <span class="muted">→ {customLabels[c.id] ?? ''}</span>
          </div>
        </div>
        {#if errors[c.id + ':shortcut']}<p class="error">{errors[c.id + ':shortcut']}</p>{/if}
      </div>
    {/each}
    {#if customs.length === 0}<p class="muted">{t('Aucune commande à toi pour l’instant.')}</p>{/if}
    <div class="row">
      <button onclick={() => openTransfer('export')} disabled={customs.length === 0 || transfer !== null}>{t('Exporter mes commandes…')}</button>
      <button onclick={() => openTransfer('import')} disabled={transfer !== null}>{t('Importer des commandes…')}</button>
    </div>
    {#if transfer?.mode === 'export'}
      <div class="command">
        <p>{t('Le fichier (.scriibe.json) contient tes {n} commande(s) : titre, phrases et raccourci.', { n: customs.length })} {exportWhere(status.os)} {t('Le fichier n’est pas chiffré : relis-le avant de le partager.')}</p>
        {#if transfer.message}<p role="status"><b>{transfer.message}</b></p>{/if}
        <div class="chips"><button onclick={closeTransfer}>{t('Fermer')}</button><button class="primary" disabled={transfer.busy} onclick={exportCommands}>{t('Enregistrer le fichier')}</button></div>
      </div>
    {:else if transfer?.mode === 'import'}
      <div class="command">
        {#if transfer.askPath}
          <p>{t('Sur ce système, indique le chemin du fichier .scriibe.json à importer :')}</p>
          <div class="chips"><input type="text" class="phrase" bind:value={transfer.path} placeholder="/home/…/scriibe-commandes.scriibe.json" disabled={transfer.busy} />
            <button class="primary" disabled={transfer.busy || !transfer.path.trim()} onclick={() => readImport(transfer!.path)}>{t('Lire le fichier')}</button></div>
        {:else if transfer.items.length > 0}
          <p>{t('Ce fichier contient {n} commande(s). Rien n’est modifié tant que tu n’as pas cliqué sur « Importer ». L’import ajoute : tes commandes ne sont jamais remplacées. Si le fichier vient de quelqu’un d’autre, relis-le : une commande appuie un raccourci de l’appli active quand on dit sa phrase.', { n: transfer.items.length })}</p>
          {#each transfer.items as item, i (i)}
            <label class="pickrow">
              <input type="checkbox" bind:checked={transfer.tick[i]} disabled={transfer.busy || item.problem !== null} />
              <span><b>{item.name || t('(sans titre)')}</b> · {item.shortcut || '—'}<br />
                <span class="muted">{item.phrases.map((p) => '« ' + p + ' »').join(' ')}</span>
                {#if item.dropped.length > 0}<br /><span class="warn">{t('Phrase(s) écartée(s) (déjà prises ou trop courtes) : {list}', { list: item.dropped.map((p) => '« ' + p + ' »').join(' ') })}</span>{/if}
                {#if !item.needs_selection}<br /><span class="warn">{t('Agit même sans texte sélectionné : le raccourci est appuyé à chaque fois que la phrase est dite.')}</span>{/if}
                {#if item.problem}<br /><span class="warn">{problemText(item.problem)}</span>{/if}</span>
            </label>
          {/each}
        {:else if transfer.busy}
          <p class="muted">{t('Chargement…')}</p>
        {/if}
        {#if transfer.message}<p role="status" class:warn={!transfer.done}><b>{transfer.message}</b></p>{/if}
        <div class="chips">
          <button onclick={closeTransfer}>{transfer.done ? t('Fermer') : t('Annuler')}</button>
          {#if transfer.items.length > 0}<button class="primary" disabled={transfer.busy || !transfer.tick.some(Boolean)} onclick={applyImport}>{t('Importer')}</button>{/if}
        </div>
      </div>
    {/if}
    <div class="command newcommand">
      <div class="head"><b>{t('Nouvelle commande')}</b></div>
      <div class="chips">
        <input type="text" placeholder={t('Titre (ex. : Barré)')} bind:value={customDraft.name} />
        <input type="text" class="phrase" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Phrase à dire (ex. : mettre en barré)')} bind:value={customDraft.phrase} />
        <ShortcutField value={customDraft.shortcut} onset={(s) => (customDraft.shortcut = s)} />
        <label class="inline"><input type="checkbox" bind:checked={customDraft.needsSelection} /> {t('Seulement si du texte est sélectionné')}</label>
        <button onclick={addCustom}>{t('Ajouter la commande')}</button>
      </div>
      {#if customError}<p class="error nomargin">{customError}</p>{/if}
    </div>
</div>
</Card>
