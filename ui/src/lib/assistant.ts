import type { Status } from './api';

/** Ce que l'assistant de premier lancement demande à l'API : la machine mesurée et les chaînes proposées. */
export interface AssistantChain {
  id: string;
  title: string;
  detail: string;
  timing: string;
  measured: boolean;
  needs: string[];
  needs_key: 'openrouter' | null;
}

export interface Assistant {
  done: boolean;
  machine: {
    physical_cores: number;
    logical_cores: number;
    ram_gb: number;
    gpus_known: boolean;
    gpus: { name: string; vram_gb: number | null; integrated: boolean; capable: boolean }[];
    capable_gpu: boolean;
  };
  chains: AssistantChain[];
  recommended: string;
  has_openrouter_key: boolean;
}

/** Les éléments de la page Installation, avec ce qui prouve qu'ils sont là. */
export const needLabels: Record<string, string> = {
  'whisper-server': 'whisper-server',
  'whisper-base': 'Modèle Whisper « base »',
  'whisper-small': 'Modèle Whisper « small »',
  'llama-server': 'llama-server',
  'llama-vulkan': 'llama-server (Vulkan)',
  'qwen-1.5b': 'Modèle Qwen 2.5 1,5 B',
  'qwen-3b': 'Modèle Qwen 2.5 3 B',
};

const has = (paths: string[], part: string) => paths.some((p) => p.toLowerCase().includes(part));

/** Est-ce que cet élément est déjà installé ? (Pour llama-server Vulkan, la présence du programme suffit : la version se choisit à la page Installation.) */
export function isInstalled(id: string, s: Status): boolean {
  switch (id) {
    case 'whisper-server':
      return !!s.whisper.binary;
    case 'whisper-base':
      return has(s.whisper.models, 'ggml-base');
    case 'whisper-small':
      return has(s.whisper.models, 'ggml-small');
    case 'llama-server':
    case 'llama-vulkan':
      return !!s.llama.binary;
    case 'qwen-1.5b':
      return has(s.llama.models, 'qwen2.5-1.5b');
    case 'qwen-3b':
      return has(s.llama.models, 'qwen2.5-3b');
    default:
      return false;
  }
}

/** Ce qui manque pour une chaîne : éléments à installer et clé à saisir. */
export function missing(chain: AssistantChain, s: Status): { items: string[]; key: boolean } {
  return {
    items: chain.needs.filter((id) => !isInstalled(id, s)),
    key: chain.needs_key !== null && !s.keys[chain.needs_key],
  };
}
