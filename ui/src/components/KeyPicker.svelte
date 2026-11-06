<script lang="ts">
  import { onDestroy } from 'svelte';
  import { api } from '../lib/api';
  import { settings } from '../lib/settings.svelte';
  import { status } from '../lib/status.svelte';
  import { keyName, t } from '../lib/i18n.svelte';

  interface Capture {
    active: boolean;
    kind?: 'dictation' | 'translate';
    finished?: boolean;
    labels?: string[];
    ids?: string[] | null;
    error?: string | null;
  }

  let s = $derived(status.value);
  let v = $derived(settings.value);

  let kind = $state<'dictation' | 'translate' | null>(null);
  let capture = $state<Capture>({ active: false });
  let waited = $state(0);
  let timer: ReturnType<typeof setInterval> | null = null;
  let testing = $state(false);
  let detected = $state(false);

  let mac = $derived(status.os === 'macos');
  /** La touche de dictée par défaut du système (Ctrl droit sous Windows, Option droite sur Mac) : on la rétablit en vidant la liste. */
  let defaultId = $derived(s?.default_dictation_id ?? 'right_control');

  /** Les touches de modification seules (sauf celles qui écoutent tout de suite) ne commencent à écouter qu'après une courte attente. */
  let waitNote = $derived.by(() => {
    const ids = capture.finished && !capture.error ? capture.ids : null;
    if (!ids || kind !== 'dictation' || !s) return false;
    const modifiers = new Set(s.key_choices.filter((c) => c.modifier).map((c) => c.id));
    return ids.every((id) => modifiers.has(id)) && !(ids.length === 1 && s.immediate_keys.includes(ids[0]));
  });

  async function begin(which: 'dictation' | 'translate') {
    stop(false);
    kind = which;
    waited = 0;
    capture = { active: true };
    await api('keys/capture/start', { kind: which });
    timer = setInterval(async () => {
      capture = await api<Capture>('keys/capture');
      waited += 0.15;
      if (capture.finished && timer) {
        clearInterval(timer);
        timer = null;
      }
    }, 150);
  }

  /** Quand la fenêtre de Scriibe a le focus, le crochet du système ne reçoit rien : pendant une capture, la page rapporte elle-même les touches. */
  const pressed = new Set<string>();
  let sending: Promise<unknown> = Promise.resolve();

  function report(code: string, down: boolean) {
    sending = sending.then(() => api('keys/capture/key', { code, down })).catch(() => {});
  }

  function onKeyDown(e: KeyboardEvent) {
    if (!kind || capture.finished) return;
    e.preventDefault();
    if (e.repeat || pressed.has(e.code)) return;
    pressed.add(e.code);
    report(e.code, true);
  }

  function onKeyUp(e: KeyboardEvent) {
    if (!kind) return;
    e.preventDefault();
    if (!pressed.delete(e.code)) return;
    report(e.code, false);
  }

  /** La fenêtre perd le focus (touche Windows, Alt+Tab…) : les relâchements ne viendront pas, on les rapporte. */
  function onBlur() {
    for (const code of [...pressed]) {
      pressed.delete(code);
      if (kind) report(code, false);
    }
  }

  function stop(callServer = true) {
    pressed.clear();
    if (timer) clearInterval(timer);
    timer = null;
    if (callServer) void api('keys/capture/stop', {});
    kind = null;
    capture = { active: false };
  }

  async function use() {
    if (!capture.ids || !kind) return;
    const ids = capture.ids;
    const ok =
      kind === 'dictation'
        ? await settings.change({ dictationKeys: ids.length === 1 && ids[0] === defaultId ? [] : ids })
        : await settings.change({ translateKeys: ids });
    if (ok) stop();
  }

  async function reset() {
    stop();
    await settings.change({ dictationKeys: [], translateKeys: [] });
  }

  /** Quinze secondes pendant lesquelles on relit l'état toutes les 0,3 s : la touche maintenue doit faire passer Scriibe en écoute. */
  async function test() {
    stop();
    testing = true;
    detected = false;
    const end = Date.now() + 15000;
    while (Date.now() < end && !detected) {
      await status.refresh();
      if (status.value?.recording) detected = true;
      else await new Promise((r) => setTimeout(r, 300));
    }
    testing = false;
  }

  onDestroy(() => {
    if (timer) clearInterval(timer);
    if (kind) void api('keys/capture/stop', {});
  });
</script>

<svelte:window onkeydown={onKeyDown} onkeyup={onKeyUp} onblur={onBlur} />

{#if s && v}
  <div class="row">
    <span>{t('Touche de dictée :')} <b>{keyName(s.dictation_label)}</b></span>
    <button onclick={() => begin('dictation')} disabled={kind === 'dictation'}>{t('Changer…')}</button>
  </div>
  <div class="row">
    <span>{t('Touche tenue en plus pour traduire :')} <b>{keyName(s.translate_label)}</b></span>
    <button onclick={() => begin('translate')} disabled={kind === 'translate'}>{t('Changer…')}</button>
  </div>

  {#if kind}
    <div class="capture" role="status">
      {#if !capture.finished}
        <b>{kind === 'dictation' ? t('Appuie sur la touche ou la combinaison voulue, puis relâche tout.') : t('Appuie sur la touche à tenir en plus, puis relâche tout.')}</b>
        {#if capture.labels?.length}<div class="live">{capture.labels.map(keyName).join(' + ')}</div>
        {:else if waited > 6}
          <div class="warn">
            {#if mac}
              {t('Aucune touche reçue. Vérifie que Scriibe a l’autorisation « Surveillance de l’entrée » (Réglages Système → Confidentialité et sécurité), puis relance Scriibe. Sinon, essaie une autre touche.')}
            {:else}
              {t('Aucune touche reçue. Sur la plupart des portables, la touche Fn est gérée par le clavier lui-même : Windows ne la voit pas. Essaie une autre touche (Windows, Ctrl gauche, une touche de fonction…). Les touches Windows et Alt gauche se captent aussi depuis une autre fenêtre : passe dans le Bloc-notes, appuie sur la touche, puis reviens ici.')}
            {/if}
          </div>
        {/if}
        <div class="row"><button onclick={() => stop()}>{t('Annuler')}</button></div>
      {:else if capture.error}
        <div class="bad">{capture.labels?.map(keyName).join(' + ')} — {t(capture.error)}</div>
        <div class="row"><button onclick={() => begin(kind!)}>{t('Réessayer')}</button><button onclick={() => stop()}>{t('Annuler')}</button></div>
      {:else}
        <div>{t('Nouvelle touche :')} <b>{capture.labels?.map(keyName).join(' + ')}</b></div>
        {#if waitNote}
          <p class="muted">{t('Ce sont des touches de modification seules : il faudra les tenir une fraction de seconde (0,3 s) avant que l’écoute commence, pour qu’un raccourci clavier (Ctrl+C, Maj+lettre…) n’ouvre pas le micro.')}</p>
        {/if}
        <div class="row"><button class="primary" onclick={use}>{t('Utiliser cette touche')}</button><button onclick={() => begin(kind!)}>{t('Recommencer')}</button><button onclick={() => stop()}>{t('Annuler')}</button></div>
      {/if}
    </div>
  {/if}

  <div class="row">
    <button onclick={test} disabled={testing || !!kind}>{testing ? t('Maintiens ta touche…') : t('Tester ma touche')}</button>
    <button onclick={reset} disabled={!v.dictationKeys.length && !v.translateKeys.length}>{t('Rétablir {keys}', { keys: keyName(`${s.default_dictation_label} + ${s.default_translate_label}`) })}</button>
    {#if detected}<span class="ok">{t('✔ Touche reconnue : Scriibe écoute ({key}).', { key: keyName(s.dictation_label) })}</span>{:else if testing}<span class="muted">{mac ? t('Passe dans TextEdit (ou une autre appli), puis maintiens {key}…', { key: keyName(s.dictation_label) }) : t('Passe dans le Bloc-notes (ou une autre appli), puis maintiens {key}…', { key: keyName(s.dictation_label) })}</span>{/if}
  </div>
  <details class="muted">
    <summary>{t('Quelles touches choisir ?')}</summary>
    {#if mac}
      {t('Une combinaison se tient en entier (par exemple Option gauche + F9) : l’une des touches seule ne fait rien. Les touches Option, Commande (⌘), Fn, Ctrl, Maj, les touches de fonction (F1 à F20), Début, Fin, Page préc. et Page suiv. sont possibles ; les lettres, chiffres, Espace, Entrée et les flèches ne le sont pas. Sur la plupart des Mac, F1 à F12 demandent Fn en plus, sauf si les « touches de fonction standard » sont activées. Une touche ordinaire de la combinaison (F9…) n’est pas retenue : l’appli active la reçoit aussi.')}
    {:else}
      {t('Une combinaison se tient en entier (par exemple Ctrl gauche + F9) : l’une des touches seule ne fait rien. Les touches Windows, Alt gauche, Maj gauche, Ctrl, les touches de fonction, « < > » (à gauche de W), Pause, Inser… sont possibles ; les lettres, chiffres, Espace, Entrée et AltGr (nécessaire à « @ » et « € » sur un clavier AZERTY) ne le sont pas. Une touche ordinaire de la combinaison (F9, « < > ») n’écrit rien dans l’appli pendant la dictée.')}
    {/if}
  </details>
{/if}

<style>
  .capture { margin: 8px 0; padding: 10px 12px; border-radius: 10px; background: var(--bg); border: 1px dashed var(--line); }
  .live { margin: 8px 0; font-size: 1.2rem; font-weight: 600; }
</style>
