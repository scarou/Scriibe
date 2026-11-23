<script lang="ts">
  import Card from '../components/Card.svelte';
  import { status } from '../lib/status.svelte';
  import { exportWhere } from '../lib/exportwhere';
  import HelpButton from '../components/HelpButton.svelte';
  import { uiState } from '../lib/uiState';
  import PageHeader from '../components/PageHeader.svelte';
  import { onMount, tick } from 'svelte';
  import { nav } from '../lib/nav.svelte';
  import { api, type Dictionary, type Stats } from '../lib/api';
  import { settings } from '../lib/settings.svelte';
  import { saveError, storedDictionary, withCorrection, type SaveResult } from '../lib/dictionary';
  import { i18n, t, tx } from '../lib/i18n.svelte';

  let words = $state<string[]>([]);
  let corrections = $state<{ heard: string; written: string }[]>([]);
  let newWord = $state('');
  let heard = $state('');
  let written = $state('');
  let writtenInput = $state<HTMLInputElement>();
  let saved = $state('');
  let loaded = $state(false);
  let allSigns = $state<{ said: string; written: string; langs: 'fr' | 'en' | 'both' }[]>([]);
  /** Seuls les signes de la langue de l'interface (et les symboles communs) s'affichent ; les autres restent actifs en dictée dans leur langue. */
  let signs = $derived(allSigns.filter((s) => s.langs === 'both' || s.langs === i18n.lang));
  let weekCorrections = $state(0);
  /** Filtres de recherche des deux listes (ne touchent pas au dictionnaire enregistré). */
  let filterWords = $state('');
  let filterCorrections = $state('');
  const norm = (x: string) => x.toLowerCase();
  let shownWords = $derived(words.map((w, i) => ({ w, i })).filter(({ w }) => norm(w).includes(norm(filterWords.trim()))));
  let shownCorrections = $derived(
    corrections.map((c, i) => ({ c, i })).filter(({ c }) => `${norm(c.heard)} ${norm(c.written)}`.includes(norm(filterCorrections.trim()))),
  );

  /** Tri alphabétique des mots (A → Z, majuscules et accents ignorés), enregistré comme le reste. */
  function sortWords() {
    words = [...words].sort((a, b) => a.localeCompare(b, i18n.lang, { sensitivity: 'base' }));
    changed();
  }
  function sortCorrections(by: 'heard' | 'written') {
    corrections = [...corrections].sort((a, b) => a[by].localeCompare(b[by], i18n.lang, { sensitivity: 'base' }));
    changed();
  }
  /** Un mot modifié sur place ; vidé, il est retiré. */
  function editWord(i: number, value: string) {
    const v = value.trim();
    words = v ? words.map((w, j) => (j === i ? v : w)) : words.filter((_, j) => j !== i);
    changed();
  }
  /** Une correction modifiée sur place ; si l'un des deux côtés est vidé, elle est retirée. */
  function editCorrection(i: number, side: 'heard' | 'written', value: string) {
    const v = value.trim();
    const next = { ...corrections[i], [side]: v };
    corrections = next.heard && next.written ? corrections.map((c, j) => (j === i ? next : c)) : corrections.filter((_, j) => j !== i);
    changed();
  }
  let v = $derived(settings.value);

  // Commande vocale « ajouter au dictionnaire » : le texte sélectionné arrive dans « Entendu », le curseur va dans « Écrit ».
  $effect(() => {
    const prefill = nav.dictionaryPrefill;
    if (prefill === null) return;
    nav.dictionaryPrefill = null;
    heard = prefill;
    written = '';
    void tick().then(() => writtenInput?.focus());
  });

  /** Hauteur commune des deux cartes (px), réglée par la poignée ; `null` = hauteur naturelle. Retenue d'un lancement à l'autre (fichier d'états de Scriibe). */
  const MIN_H = 180;
  const MAX_H = 1400;
  function savedHeight(): number | null {
    const n = uiState.state.dictionaryHeight;
    return n !== undefined && n >= MIN_H && n <= MAX_H ? n : null;
  }
  let height = $state<number | null>(savedHeight());
  let pair = $state<HTMLElement>();
  let drag: { y: number; h: number } | null = null;

  function setHeight(h: number | null) {
    height = h === null ? null : Math.min(MAX_H, Math.max(MIN_H, Math.round(h)));
    // Pendant le glissement de la poignée les changements s'enchaînent : un seul envoi au repos.
    uiState.save({ dictionaryHeight: height }, 300);
  }
  function dragStart(e: PointerEvent) {
    drag = { y: e.clientY, h: height ?? pair?.getBoundingClientRect().height ?? MIN_H };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function dragMove(e: PointerEvent) {
    if (drag) setHeight(drag.h + e.clientY - drag.y);
  }
  function dragEnd() {
    drag = null;
  }
  function handleKey(e: KeyboardEvent) {
    const now = height ?? pair?.getBoundingClientRect().height ?? MIN_H;
    if (e.key === 'ArrowDown') setHeight(now + 24);
    else if (e.key === 'ArrowUp') setHeight(now - 24);
    else if (e.key === 'Home' || e.key === 'Escape') setHeight(null);
    else return;
    e.preventDefault();
  }

  const bullets = [
    { id: 'dash', symbol: '-', label: tx('Tiret  -') },
    { id: 'dot', symbol: '•', label: tx('Pastille  •') },
    { id: 'star', symbol: '*', label: tx('Étoile  *') },
  ];
  let symbol = $derived(bullets.find((b) => b.id === v?.listBullet)?.symbol ?? '-');
  let listSayings = $derived([
    { said: 'tiret · puce', example: `${symbol} du lait` },
    { said: 'point un · point 2…', example: '1. 2. 3.' },
    { said: 'numéro suivant', example: 'le numéro d’après' },
  ]);

  /** Les retours à la ligne et espaces insécables ne se voient pas : on les montre. */
  function shown(written: string): string {
    if (written === '\n') return '↵';
    if (written === '\n\n') return '¶';
    if (written === '.\n') return '. ↵';
    return written.replaceAll('\u00a0', '');
  }
  let timer: ReturnType<typeof setTimeout> | undefined;

  function apply(d: Dictionary) {
    words = d.words;
    corrections = d.corrections;
  }

  /** Des modifications faites dans la page, pas encore enregistrées. */
  let dirty = false;

  /** Enregistre un instant après la dernière modification. */
  function changed() {
    if (!loaded) return;
    saved = '';
    dirty = true;
    clearTimeout(timer);
    timer = setTimeout(flush, 500);
  }

  /** Enregistre tout de suite ce qui attend : avant un export ou un import, pour que le fichier et le serveur voient la même chose, et que
   *  l'enregistrement automatique ne vienne pas écraser un import après coup. */
  async function flush() {
    clearTimeout(timer);
    if (!dirty) return;
    const out = await api<SaveResult>('dictionary', { words, corrections });
    const stored = storedDictionary(out);
    if (stored) {
      dirty = false;
      apply(stored);
      saved = t('Enregistré ✔');
    } else {
      // Refusé (fichier illisible…) : la page garde ce que la personne a tapé, le dit, et ne remplace rien par la réponse d'erreur.
      saved = t(saveError(out) ?? 'enregistrement impossible');
    }
  }

  /** L'export : aperçu, catégories à cocher et avertissement avant que le serveur ne demande où enregistrer. */
  let exporting = $state<{ words: boolean; corrections: boolean; busy: boolean; result: string } | null>(null);

  async function openExport() {
    await closeImport();
    await flush();
    exporting = { words: words.length > 0, corrections: corrections.length > 0, busy: false, result: '' };
  }

  async function exportDictionary() {
    if (!exporting) return;
    exporting.busy = true;
    exporting.result = '';
    const out = await api<{ ok: boolean; saved?: boolean; path?: string; error?: string }>('dictionary/export', {
      words: exporting.words,
      corrections: exporting.corrections,
    });
    exporting.busy = false;
    exporting.result = !out.ok
      ? t(out.error ?? 'enregistrement impossible : l’emplacement choisi n’est pas accessible en écriture')
      : out.saved
        ? t('Enregistré : {path}', { path: out.path ?? '' })
        : t('Export annulé : rien n’a été enregistré.');
  }

  /** L'import : le serveur lit et contrôle le fichier, la page montre l'aperçu, rien n'est écrit avant l'accord ; la fusion ajoute sans remplacer. */
  type ImportStatus = { status: 'new' | 'same' | 'conflict'; existing?: string };
  type ImportedWord = ImportStatus & { word: string };
  type ImportedCorrection = ImportStatus & { heard: string; written: string };
  let importing = $state<{
    words: ImportedWord[];
    corrections: ImportedCorrection[];
    tickWords: boolean[];
    tickCorrections: boolean[];
    token: string;
    askPath: boolean;
    path: string;
    busy: boolean;
    message: string;
    done: boolean;
  } | null>(null);

  const emptyImport = () => ({ words: [], corrections: [], tickWords: [], tickCorrections: [], token: '', askPath: false, path: '', busy: true, message: '', done: false });

  async function readImport(path?: string) {
    exporting = null;
    await flush();
    if (!importing) importing = emptyImport();
    importing.busy = true;
    importing.message = '';
    const out = await api<{ ok: boolean; cancelled?: boolean; ask_path?: boolean; error?: string; token?: string; words?: ImportedWord[]; corrections?: ImportedCorrection[] }>(
      'dictionary/import/read',
      path ? { path } : {},
    );
    if (!importing) return;
    importing.busy = false;
    if (out.ok && out.cancelled) importing = null;
    else if (out.ok && out.words && out.corrections) {
      importing.askPath = false;
      importing.words = out.words;
      importing.corrections = out.corrections;
      importing.token = out.token ?? '';
      importing.tickWords = out.words.map((w) => w.status === 'new');
      importing.tickCorrections = out.corrections.map((c) => c.status === 'new');
    } else if (out.ask_path) importing.askPath = true;
    else importing.message = t(out.error ?? "le fichier n'a pas pu être lu");
  }

  const toAdd = (i: NonNullable<typeof importing>) => i.tickWords.filter(Boolean).length + i.tickCorrections.filter(Boolean).length;

  async function applyImport() {
    if (!importing) return;
    importing.busy = true;
    await flush();
    const out = await api<{ ok: boolean; words_added?: number; corrections_added?: number; already?: number; conflicts?: number; over_limit?: number; error?: string }>('dictionary/import/apply', {
      token: importing.token,
      words: importing.tickWords,
      corrections: importing.tickCorrections,
    });
    importing.busy = false;
    if (!out.ok) {
      importing.message = t(out.error ?? "le fichier n'a pas pu être lu");
      return;
    }
    importing.words = [];
    importing.corrections = [];
    importing.done = true;
    importing.message = t('Importé : {words} mot(s) et {corrections} correction(s) ajouté(s).', { words: out.words_added ?? 0, corrections: out.corrections_added ?? 0 });
    const left = (out.already ?? 0) + (out.conflicts ?? 0);
    if (left > 0) importing.message += ' ' + t('{n} élément(s) étaient déjà présents : ils n’ont pas été touchés.', { n: left });
    if ((out.over_limit ?? 0) > 0) importing.message += ' ' + t('{n} mot(s) n’ont pas été ajoutés : le dictionnaire est plein (1 000 mots au plus).', { n: out.over_limit ?? 0 });
    // Le serveur a écrit le dictionnaire : la page le relit, pour que son enregistrement automatique ne l'écrase pas.
    clearTimeout(timer);
    dirty = false;
    apply(await api<Dictionary>('dictionary'));
  }

  async function closeImport() {
    if (importing && !importing.done) await api('dictionary/import/cancel', {});
    importing = null;
  }

  /** Tableur : le dictionnaire en CSV pour filtrer, trier et regrouper ; le CSV modifié le remplace après aperçu de ce qui change. */
  type SheetPreview = {
    token: string;
    words_total: number;
    corrections_total: number;
    words_added: string[];
    words_removed: string[];
    corrections_added: { heard: string; written: string }[];
    corrections_removed: { heard: string; written: string }[];
    corrections_changed: { heard: string; before: string; after: string }[];
  };
  let sheet = $state<{ preview: SheetPreview | null; askPath: boolean; path: string; busy: boolean; message: string; done: boolean } | null>(null);
  const sheetChanges = (p: SheetPreview) =>
    p.words_added.length + p.words_removed.length + p.corrections_added.length + p.corrections_removed.length + p.corrections_changed.length;

  async function sheetExport() {
    await flush();
    sheet = { preview: null, askPath: false, path: '', busy: true, message: '', done: false };
    const out = await api<{ ok: boolean; saved?: boolean; path?: string; error?: string }>('dictionary/sheet/export', {});
    if (!sheet) return;
    sheet.busy = false;
    sheet.message = !out.ok
      ? t(out.error ?? 'enregistrement impossible : l’emplacement choisi n’est pas accessible en écriture')
      : out.saved
        ? t('Enregistré : {path}', { path: out.path ?? '' }) + ' ' + t('Ouvre-le dans ton tableur, modifie-le, enregistre-le en CSV, puis reprends-le avec « Reprendre le CSV modifié… ».')
        : t('Export annulé : rien n’a été enregistré.');
    sheet.done = true;
  }

  async function sheetRead(path?: string) {
    exporting = null;
    await closeImport();
    await flush();
    sheet = { preview: null, askPath: false, path: path ?? '', busy: true, message: '', done: false };
    const out = await api<{ ok: boolean; cancelled?: boolean; ask_path?: boolean; error?: string } & Partial<SheetPreview>>('dictionary/sheet/read', path ? { path } : {});
    if (!sheet) return;
    sheet.busy = false;
    if (out.ok && out.cancelled) sheet = null;
    else if (out.ok && out.token) sheet.preview = out as SheetPreview;
    else if (out.ask_path) sheet.askPath = true;
    else sheet.message = t(out.error ?? "le fichier n'a pas pu être lu");
  }

  async function sheetApply() {
    if (!sheet?.preview) return;
    sheet.busy = true;
    await flush();
    const out = await api<SaveResult & { token?: string }>('dictionary/sheet/apply', { token: sheet.preview.token });
    const stored = storedDictionary(out);
    if (!sheet) return;
    sheet.busy = false;
    if (!stored) {
      sheet.message = t(saveError(out) ?? "le fichier n'a pas pu être lu");
      return;
    }
    clearTimeout(timer);
    dirty = false;
    apply(stored);
    sheet.preview = null;
    sheet.done = true;
    sheet.message = t('Dictionnaire remplacé : {words} mot(s) et {corrections} correction(s).', { words: stored.words.length, corrections: stored.corrections.length });
  }

  async function sheetClose() {
    if (sheet && !sheet.done) await api('dictionary/sheet/cancel', {});
    sheet = null;
  }

  function addWord() {
    const w = newWord.trim();
    if (!w) return;
    words = [...words, w];
    newWord = '';
    changed();
  }

  function addCorrection() {
    if (!heard.trim() || !written.trim()) return;
    corrections = withCorrection(corrections, { heard: heard.trim(), written: written.trim() });
    heard = '';
    written = '';
    changed();
  }

  onMount(() => {
    void settings.load();
    void api<Stats>('stats').then((r) => (weekCorrections = r.corrections_this_week));
    void api<{ signs: { said: string; written: string; langs: 'fr' | 'en' | 'both' }[] }>('signs').then((out) => (allSigns = out.signs));
    void api<Dictionary>('dictionary').then((d) => {
      apply(d);
      loaded = true;
    });
    return () => clearTimeout(timer);
  });

  /** Demande de la carte « Sauvegarder et transférer » : la même carte d'export ou d'import que les boutons de la page. */
  $effect(() => {
    const kind = nav.transferRequest;
    if (kind !== 'dictionary-export' && kind !== 'dictionary-import') return;
    if (!loaded) return;
    nav.transferRequest = null;
    if (kind === 'dictionary-import') void readImport();
    else if (words.length > 0 || corrections.length > 0) void openExport();
    else nav.transferNothing(t('Le dictionnaire est vide : rien à exporter.'));
  });
</script>

<PageHeader title={t('Dictionnaire')}>{t('Ce que Scriibe écrit exactement, même quand Whisper l’entend autrement.')} {saved}</PageHeader>

<div class="card figures">
  <div><b>{words.length}</b> <span class="muted">{t('mots dans ton dictionnaire')}</span></div>
  <div><b>{weekCorrections}</b> <span class="muted">{t('corrections du dictionnaire cette semaine')}</span></div>
</div>

<div class="two" class:sized={height !== null} style:height={height === null ? undefined : `${height}px`} bind:this={pair}>

<Card id="dictionary-mots-a-ecrire-exactement" title={t('Mots à écrire exactement')} help="dictionary" intro={t('Noms propres, sigles, marques : « Claude Code », « to-do »… Whisper les entend parfois en deux mots ou sans majuscule.')}>
  <div class="row">
    <input type="text" class="grow" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Ajouter un mot ou une expression')} bind:value={newWord} disabled={!!importing?.busy} onkeydown={(e) => e.key === 'Enter' && addWord()} />
    <button onclick={addWord} disabled={!newWord.trim() || !!importing?.busy}>{t('Ajouter')}</button>
  </div>
  {#if words.length > 1}
    <div class="row">
      <input type="search" class="grow" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Filtrer les mots')} bind:value={filterWords} />
      <button onclick={sortWords} title={t('Trier de A à Z')}>{t('Trier A → Z')}</button>
    </div>
  {/if}
  <div class="scroll">
    {#each shownWords as { w, i } (i)}
      <div class="edit-row">
        <input type="text" class="grow cell" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" aria-label={t('Mot')} value={w} onchange={(e) => editWord(i, e.currentTarget.value)} />
        <button class="x" title={t('Retirer')} aria-label={t('Retirer')} onclick={() => { words = words.filter((_, j) => j !== i); changed(); }}>×</button>
      </div>
    {:else}
      <span class="muted">{words.length ? t('Aucun mot ne correspond au filtre.') : t('Aucun mot pour l’instant.')}</span>
    {/each}
  </div>
</Card>

<Card id="dictionary-corrections-entendu-ecrit" title={t('Corrections « entendu → écrit »')} help="dictionary" intro={t('Quand Whisper entend toujours « cri be », on écrit « Scriibe ». Mot entier, majuscules ignorées.')}>
  <div class="row">
    <input type="text" class="grow" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Entendu')} bind:value={heard} disabled={!!importing?.busy} />
    <span>→</span>
    <input type="text" class="grow" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Écrit')} bind:this={writtenInput} bind:value={written} disabled={!!importing?.busy} onkeydown={(e) => e.key === 'Enter' && addCorrection()} />
    <button onclick={addCorrection} disabled={!heard.trim() || !written.trim() || !!importing?.busy}>{t('Ajouter')}</button>
  </div>
  {#if corrections.length}
    {#if corrections.length > 1}
      <div class="row">
        <input type="search" class="grow" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" placeholder={t('Filtrer les corrections')} bind:value={filterCorrections} />
        <button onclick={() => sortCorrections('heard')} title={t('Trier de A à Z')}>{t('Trier par « Entendu »')}</button>
        <button onclick={() => sortCorrections('written')} title={t('Trier de A à Z')}>{t('Trier par « Écrit »')}</button>
      </div>
    {/if}
    <div class="scroll">
      {#each shownCorrections as { c, i } (i)}
        <div class="edit-row">
          <input type="text" class="grow cell" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" aria-label={t('Entendu')} value={c.heard} onchange={(e) => editCorrection(i, 'heard', e.currentTarget.value)} />
          <span>→</span>
          <input type="text" class="grow cell" autocapitalize="off" autocorrect="off" autocomplete="off" spellcheck="false" aria-label={t('Écrit')} value={c.written} onchange={(e) => editCorrection(i, 'written', e.currentTarget.value)} />
          <button class="x" title={t('Retirer')} aria-label={t('Retirer')} onclick={() => { corrections = corrections.filter((_, j) => j !== i); changed(); }}>×</button>
        </div>
      {:else}
        <span class="muted">{t('Aucune correction ne correspond au filtre.')}</span>
      {/each}
    </div>
  {:else}
    <span class="muted">{t('Aucune correction pour l’instant.')}</span>
  {/if}
</Card>

</div>

<div
  class="grip"
  role="slider"
  aria-orientation="vertical"
  aria-valuemin={MIN_H}
  aria-valuemax={MAX_H}
  aria-valuenow={height ?? undefined}
  aria-label={t('Hauteur des deux cartes')}
  title={t('Glisser pour régler la hauteur des deux cartes ; double-clic pour revenir à la hauteur naturelle')}
  tabindex="0"
  onpointerdown={dragStart}
  onpointermove={dragMove}
  onpointerup={dragEnd}
  onpointercancel={dragEnd}
  ondblclick={() => setHeight(null)}
  onkeydown={handleKey}
><span></span></div>

<Card id="dictionary-export-import" title={t('Exporter et importer le dictionnaire')} help="dictionary" intro={t('Sauvegarder ton vocabulaire dans un fichier, le reprendre sur un autre ordinateur ou le partager.')}>
  <div class="row">
    <button onclick={openExport} disabled={exporting !== null || (words.length === 0 && corrections.length === 0)}>{t('Exporter…')}</button>
    <button onclick={() => readImport()} disabled={importing !== null}>{t('Importer…')}</button>
  </div>
</Card>

<Card id="dictionary-tableur" title={t('Modifier dans un tableur')} help="dictionary" intro={t('Pour filtrer, trier, regrouper ou corriger beaucoup d’entrées d’un coup : le dictionnaire s’exporte en CSV (Excel, Numbers, LibreOffice), puis le fichier modifié le remplace.')}>
  <div class="row">
    <button onclick={sheetExport} disabled={sheet !== null || (words.length === 0 && corrections.length === 0)}>{t('Exporter en CSV…')}</button>
    <button onclick={() => sheetRead()} disabled={sheet !== null}>{t('Reprendre le CSV modifié…')}</button>
  </div>
</Card>

{#if sheet}
  <Card id="dictionary-tableur-suite" accent title={t('Dictionnaire et tableur')} help="dictionary">
    {#if sheet.askPath}
      <p>{t('Sur ce système, indique le chemin du fichier .csv à reprendre :')}</p>
      <div class="row"><input type="text" class="grow" bind:value={sheet.path} placeholder="/home/…/scriibe-dictionnaire.csv" disabled={sheet.busy} />
        <button class="primary" disabled={sheet.busy || !sheet.path.trim()} onclick={() => sheetRead(sheet!.path)}>{t('Lire le fichier')}</button>
        <button onclick={sheetClose}>{t('Annuler')}</button></div>
    {:else if sheet.preview}
      {@const p = sheet.preview}
      <p>{t('Ce fichier contient {words} mot(s) et {corrections} correction(s). Il va remplacer ton dictionnaire : ce qui n’y figure plus sera retiré. Rien n’est modifié tant que tu n’as pas cliqué sur « Remplacer le dictionnaire ».', { words: p.words_total, corrections: p.corrections_total })}</p>
      {#if sheetChanges(p) === 0}
        <p class="muted">{t('Aucune différence avec ton dictionnaire actuel.')}</p>
      {:else}
        {#if p.words_removed.length + p.corrections_removed.length > 0}
          <details open><summary class="warn">{t('{n} élément(s) retiré(s)', { n: p.words_removed.length + p.corrections_removed.length })}</summary>
            <div class="scroll">
              {#each p.words_removed as w, i (i)}<div>− {w}</div>{/each}
              {#each p.corrections_removed as c, i (i)}<div>− {c.heard} → <b>{c.written}</b></div>{/each}
            </div></details>
        {/if}
        {#if p.corrections_changed.length > 0}
          <details open><summary>{t('{n} correction(s) modifiée(s)', { n: p.corrections_changed.length })}</summary>
            <div class="scroll">{#each p.corrections_changed as c, i (i)}<div>{c.heard} : {c.before} → <b>{c.after}</b></div>{/each}</div></details>
        {/if}
        {#if p.words_added.length + p.corrections_added.length > 0}
          <details><summary>{t('{n} élément(s) ajouté(s)', { n: p.words_added.length + p.corrections_added.length })}</summary>
            <div class="scroll">
              {#each p.words_added as w, i (i)}<div>+ {w}</div>{/each}
              {#each p.corrections_added as c, i (i)}<div>+ {c.heard} → <b>{c.written}</b></div>{/each}
            </div></details>
        {/if}
      {/if}
    {:else if sheet.busy}
      <p class="muted">{t('Chargement…')}</p>
    {/if}
    {#if sheet.message}<p role="status" class:warn={!sheet.done}><b>{sheet.message}</b></p>{/if}
    <div class="row">
      <span class="grow"></span>
      <button onclick={sheetClose}>{sheet.done ? t('Fermer') : t('Annuler')}</button>
      {#if sheet.preview}
        <button class="primary" disabled={sheet.busy || sheetChanges(sheet.preview) === 0} onclick={sheetApply}>{t('Remplacer le dictionnaire')}</button>
      {/if}
    </div>
  </Card>
{/if}

{#if exporting}
  <Card id="dictionary-export" accent title={t('Exporter le dictionnaire')} help="dictionary">
    <p>{t('Le fichier (.scriibe.json) contient les catégories cochées.')} {exportWhere(status.os)}</p>
    <div class="row"><label><input type="checkbox" bind:checked={exporting.words} disabled={exporting.busy || words.length === 0} /> {t('{n} mot(s)', { n: words.length })}</label></div>
    {#if words.length > 0}<details><summary>{t('Voir les mots')}</summary><div class="chips scroll">{#each words as w, i (w + i)}<span class="sign">{w}</span>{/each}</div></details>{/if}
    <div class="row"><label><input type="checkbox" bind:checked={exporting.corrections} disabled={exporting.busy || corrections.length === 0} /> {t('{n} correction(s)', { n: corrections.length })}</label></div>
    {#if corrections.length > 0}<details><summary>{t('Voir les corrections')}</summary><div class="scroll"><table><tbody>{#each corrections as c, i (c.heard + i)}<tr><td>{c.heard}</td><td>→</td><td><b>{c.written}</b></td></tr>{/each}</tbody></table></div></details>{/if}
    <p class="warn">{t('Attention : le fichier n’est pas chiffré et peut contenir des noms propres, des adresses ou d’autres textes personnels. Relis-le avant de le partager.')}</p>
    {#if exporting.result}<p role="status"><b>{exporting.result}</b></p>{/if}
    <div class="row">
      <span class="grow"></span>
      <button onclick={() => (exporting = null)}>{t('Fermer')}</button>
      <button class="primary" disabled={exporting.busy || !(exporting.words || exporting.corrections)} onclick={exportDictionary}>{t('Enregistrer le fichier')}</button>
    </div>
  </Card>
{/if}

{#if importing}
  <Card id="dictionary-import" accent title={t('Importer un dictionnaire')} help="dictionary">
    {#if importing.askPath}
      <p>{t('Sur ce système, indique le chemin du fichier .scriibe.json à importer :')}</p>
      <div class="row"><input type="text" class="grow" bind:value={importing.path} placeholder="/home/…/scriibe-dictionnaire.scriibe.json" disabled={importing.busy} />
        <button class="primary" disabled={importing.busy || !importing.path.trim()} onclick={() => readImport(importing!.path)}>{t('Lire le fichier')}</button>
        <button onclick={closeImport}>{t('Annuler')}</button></div>
    {:else if importing.words.length + importing.corrections.length > 0}
      <p>{t('Ce fichier contient {words} mot(s) et {corrections} correction(s). Rien n’est modifié tant que tu n’as pas cliqué sur « Importer ». L’import ajoute : ce qui est déjà dans ton dictionnaire n’est jamais remplacé. Si le fichier vient de quelqu’un d’autre, relis-le : les mots sont donnés au modèle de mise au propre à chaque dictée, et une correction remplace un mot par un autre dans toutes tes dictées.', { words: importing.words.length, corrections: importing.corrections.length })}</p>
      {#if importing.words.length > 0}
        <h3>{t('Mots')}</h3>
        <div class="row"><button onclick={() => (importing!.tickWords = importing!.words.map((w) => w.status === 'new'))}>{t('Tout cocher')}</button><button onclick={() => (importing!.tickWords = importing!.words.map(() => false))}>{t('Tout décocher')}</button></div>
        <div class="scroll">
          {#each importing.words as item, i (i)}
            <label class="pick">
              <input type="checkbox" bind:checked={importing.tickWords[i]} disabled={importing.busy || item.status !== 'new'} />
              <span>{item.word}{#if item.status !== 'new'} <span class="muted">· {t('déjà dans ton dictionnaire')}</span>{/if}</span>
            </label>
          {/each}
        </div>
      {/if}
      {#if importing.corrections.length > 0}
        <h3>{t('Corrections')}</h3>
        <div class="row"><button onclick={() => (importing!.tickCorrections = importing!.corrections.map((c) => c.status === 'new'))}>{t('Tout cocher')}</button><button onclick={() => (importing!.tickCorrections = importing!.corrections.map(() => false))}>{t('Tout décocher')}</button></div>
        <div class="scroll">
          {#each importing.corrections as item, i (i)}
            <label class="pick">
              <input type="checkbox" bind:checked={importing.tickCorrections[i]} disabled={importing.busy || item.status !== 'new'} />
              <span>{item.heard} → <b>{item.written}</b>
                {#if item.status === 'same'}<span class="muted">· {t('déjà dans ton dictionnaire')}</span>{/if}
                {#if item.status === 'conflict'}<span class="warn">· {t('« {heard} » est déjà corrigé en « {existing} » : la correction en place est gardée.', { heard: item.heard, existing: item.existing ?? '' })}</span>{/if}
              </span>
            </label>
          {/each}
        </div>
      {/if}
    {:else if importing.busy}
      <p class="muted">{t('Chargement…')}</p>
    {/if}
    {#if importing.message}<p role="status" class:warn={!importing.done}><b>{importing.message}</b></p>{/if}
    <div class="row">
      <span class="grow"></span>
      <button onclick={closeImport}>{importing.done ? t('Fermer') : t('Annuler')}</button>
      {#if importing.words.length + importing.corrections.length > 0}
        <button class="primary" disabled={importing.busy || toAdd(importing) === 0} onclick={applyImport}>{t('Importer')}</button>
      {/if}
    </div>
  </Card>
{/if}

<Card id="dictionary-signes-dictes" title={t('Signes dictés')} help="dictionary" intro={t('Toujours reconnus, avec leurs variantes (« ouvre » ou « ouvrez », « ferme » ou « fermez la parenthèse », « retour » ou « passe à la ligne »…). Pour un signe à toi, ajoute une correction : « trois points d’exclamation » → !!!')}>
  <div class="chips">
    {#each signs as sign (sign.said)}
      <span class="sign">{sign.said} → <b>{shown(sign.written)}</b></span>
    {/each}
  </div>
  <h3>{t('Listes')}</h3>
  <p class="muted">{t('Pour dicter une liste : dis « retour à la ligne », puis « tiret » ou « puce » devant chaque élément, ou « point un », « numéro suivant »… Reconnus seulement en début de ligne : « un tiret entre les deux mots » reste du texte.')}</p>
  <div class="chips">
    {#each listSayings as saying (saying.said)}
      <span class="sign">{saying.said} → <b>{saying.example}</b></span>
    {/each}
  </div>
  {#if v}
    <div class="row">
      <label>{t('Caractère devant chaque élément :')}
        <select value={v.listBullet} onchange={(e) => settings.change({ listBullet: e.currentTarget.value as never })}>
          {#each bullets as b (b.id)}<option value={b.id}>{t(b.label)}</option>{/each}
        </select>
      </label>
    </div>
  {/if}
</Card>


<style>
  .figures { display: flex; gap: 32px; flex-wrap: wrap; padding: 12px 18px; }
  .figures b { font-size: 1.5rem; }
  .two { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; align-items: stretch; }
  @media (max-width: 860px) { .two { grid-template-columns: 1fr; } }
  /* Hauteur réglée par la poignée : chaque carte la remplit et sa liste défile ; si une carte est repliée, la hauteur redevient naturelle. */
  .two.sized:not(:has(:global([data-folded='true']))) { grid-template-rows: minmax(0, 1fr); }
  .two.sized:not(:has(:global([data-folded='true']))) :global(.fold-card) { display: flex; flex-direction: column; min-height: 0; }
  .two.sized:not(:has(:global([data-folded='true']))) :global(.content) { flex: 1; display: flex; flex-direction: column; min-height: 0; }
  .two.sized:not(:has(:global([data-folded='true']))) .scroll { flex: 1; max-height: none; min-height: 0; }
  .two:has(:global([data-folded='true'])) { height: auto !important; }
  .grip { display: flex; justify-content: center; align-items: center; height: 18px; margin: 2px 0 10px; cursor: row-resize; touch-action: none; border-radius: 6px; }
  .grip span { width: 56px; height: 5px; border-radius: 3px; background: var(--line); }
  .grip:hover span, .grip:focus-visible span { background: var(--accent); }
  .grip:focus-visible { outline: 2px solid var(--accent); }
  /* Au-delà d'une quinzaine d'entrées, la liste défile dans sa carte au lieu d'allonger la page. */
  .scroll { max-height: 17rem; overflow-y: auto; padding-right: 6px; }
  .pick { display: flex; gap: 10px; align-items: flex-start; padding: 3px 0; cursor: pointer; }
  details { margin: 2px 0 8px 26px; }
  .sign { background: var(--bg); border: 1px solid var(--line); border-radius: 999px; padding: 2px 12px; }
  h3 { margin: 14px 0 4px; font-size: 1rem; }
  .chips { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 6px; }
  .edit-row { display: flex; align-items: center; gap: 6px; padding: 2px 0; }
  .cell { min-width: 0; }
  .x { flex: none; border: 0; background: transparent; padding: 0 8px; font-size: 1.1rem; color: var(--muted); }
  .x:hover { color: var(--bad); }
</style>
