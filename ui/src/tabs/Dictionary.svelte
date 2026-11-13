<script lang="ts">
  import Card from '../components/Card.svelte';
  import HelpButton from '../components/HelpButton.svelte';
  import VoiceCommands from '../components/VoiceCommands.svelte';
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
  let loaded = false;
  let allSigns = $state<{ said: string; written: string; langs: 'fr' | 'en' | 'both' }[]>([]);
  /** Seuls les signes de la langue de l'interface (et les symboles communs) s'affichent ; les autres restent actifs en dictée dans leur langue. */
  let signs = $derived(allSigns.filter((s) => s.langs === 'both' || s.langs === i18n.lang));
  let weekCorrections = $state(0);
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

  /** Hauteur commune des deux cartes (px), réglée par la poignée ; `null` = hauteur naturelle. Retenue d'une visite à l'autre. */
  const HEIGHT_KEY = 'dictionary:height';
  const MIN_H = 180;
  const MAX_H = 1400;
  function savedHeight(): number | null {
    try {
      const n = Number(localStorage.getItem(HEIGHT_KEY));
      return n >= MIN_H && n <= MAX_H ? n : null;
    } catch {
      return null;
    }
  }
  let height = $state<number | null>(savedHeight());
  let pair = $state<HTMLElement>();
  let drag: { y: number; h: number } | null = null;

  function setHeight(h: number | null) {
    height = h === null ? null : Math.min(MAX_H, Math.max(MIN_H, Math.round(h)));
    try {
      if (height === null) localStorage.removeItem(HEIGHT_KEY);
      else localStorage.setItem(HEIGHT_KEY, String(height));
    } catch {
      /* le stockage peut être refusé : la hauteur reste celle de la page */
    }
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
</script>

<h1>{t('Dictionnaire')}</h1>
<p class="muted">{t('Ce que Scriibe écrit exactement, même quand Whisper l’entend autrement.')} {saved}</p>

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
  <div class="chips scroll">
    {#each words as w, i (w + i)}
      <span class="chip">{w}<button class="x" title={t('Retirer')} onclick={() => { words = words.filter((_, j) => j !== i); changed(); }}>×</button></span>
    {:else}
      <span class="muted">{t('Aucun mot pour l’instant.')}</span>
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
    <div class="scroll"><table>
      <tbody>
        {#each corrections as c, i (c.heard + '→' + c.written + i)}
          <tr>
            <td>{c.heard}</td><td>→</td><td><b>{c.written}</b></td>
            <td style="text-align: right"><button class="x" title={t('Retirer')} onclick={() => { corrections = corrections.filter((_, j) => j !== i); changed(); }}>×</button></td>
          </tr>
        {/each}
      </tbody>
    </table></div>
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

{#if exporting}
  <Card id="dictionary-export" accent title={t('Exporter le dictionnaire')} help="dictionary">
    <p>{t('Le fichier (.scriibe.json) contient les catégories cochées. Une boîte te demande où l’enregistrer (sous Linux, il est placé dans le dossier Documents).')}</p>
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
        <button class="primary" disabled={importing.busy || !importing.path.trim()} onclick={() => readImport(importing!.path)}>{t('Lire le fichier')}</button></div>
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

<VoiceCommands />

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
  .chip { display: inline-flex; align-items: center; gap: 4px; background: var(--bg); border: 1px solid var(--line); border-radius: 999px; padding: 2px 4px 2px 12px; }
  .x { border: 0; background: transparent; padding: 0 8px; font-size: 1.1rem; color: var(--muted); }
  .x:hover { color: var(--bad); }
</style>
