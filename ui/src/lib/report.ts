import type { Status } from './api';
import type { BenchResult } from './benchflow';
import { base, fmt } from './format';
import { setupSummary } from './setup';

/** Le rapport que Christophe copie et colle dans la conversation : état, résultats du banc, journal. */
export function buildReport(s: Status, results: BenchResult[], devices: string[], log: string[]): string {
  let r = `Rapport Scriibe — version ${s.version}\n`;
  r += `Touche : ${s.dictation_label ?? s.hotkey} (traduction : ${s.translate_label ?? 'Maj'}) ; transcription : ${s.transcription} ; mise au propre : ${s.cleanup_label}\n`;
  r += `whisper-server : ${s.whisper.binary ? 'oui' : 'non'} ; modèles Whisper : ${s.whisper.models.map(base).join(', ') || 'aucun'}\n`;
  r += `llama-server : ${s.llama.binary ? 'oui' : 'non'} ; modèles GGUF : ${s.llama.models.map(base).join(', ') || 'aucun'}\n`;
  r += `Micros : ${s.microphones.join(' | ') || 'aucun'}${s.keys_error ? '\nTouche globale : ' + s.keys_error : ''}\n`;
  r += `llama-server : appareil choisi « ${s.llama_device || 'automatique'} » ; journal du moteur ${s.llama_log.bytes} octets\n`;
  if (s.llama_log.hints.length) r += `  lignes carte graphique :\n    ${s.llama_log.hints.join('\n    ')}\n`;
  else if (s.llama_log.head.length) r += `  (aucune ligne carte graphique) début du journal :\n    ${s.llama_log.head.join('\n    ')}\n`;
  if (devices.length) r += `llama-server --list-devices :\n  ${devices.join('\n  ')}\n`;
  r += '\n';
  if (results.length) r += `Banc de dictée — ${setupSummary(results[0].setup)}\n`;
  for (const x of results) {
    const t = x.out.timing;
    const failed = x.out.checks.filter((c) => !c.passed);
    r += `Phrase ${x.index + 1} (${x.category})${x.translated ? ' [traduction]' : ''} : ${fmt(x.take.duration)} s, micro ${x.take.verdict} (${fmt(x.take.level)})${x.take.gain > 1.05 ? ' gain ×' + fmt(x.take.gain) : ''}\n`;
    if (x.out.profile) {
      r += `  profil   : pic ${fmt(x.out.profile.peak)}, saturation ${fmt(x.out.profile.clipped * 100)} %, niveaux/¼ s : ${x.out.profile.levels.slice(0, 16).map(fmt).join(' ')}\n`;
    }
    r += `  lue      : ${x.text}\n  entendu  : ${x.out.raw ?? '—'}\n  collé    : ${x.out.text ?? '—'}\n`;
    if (t) r += `  temps    : transcription ${fmt(t.transcription)} s, mise au propre ${fmt(t.cleanup)} s (${t.outcome}), total ${fmt(t.total)} s\n`;
    if (x.out.messages.length) r += `  messages : ${x.out.messages.join(' / ')}\n`;
    r += `  contrôles: ${x.out.checks.length - failed.length}/${x.out.checks.length}${failed.length ? ' — échec : ' + failed.map((c) => c.label).join(' ; ') : ''}\n`;
  }
  r += `\nJournal (dernières lignes) :\n${log.slice(-25).join('\n')}\n`;
  return r;
}
