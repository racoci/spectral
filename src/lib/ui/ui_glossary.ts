export type UiMode = 'creative' | 'expert';

export type GlossaryKey =
  | 'holomorphic_geometry'
  | 'reassignment'
  | 'cr_residual'
  | 'stn_decomposition'
  | 'rdo_dead_zone'
  | 'inharmonicity_b'
  | 'taylor_jets';

export interface GlossaryItem {
  creative: string;
  expert: string;
  tooltip: string;
}

export const GLOSSARY_MAP: Record<GlossaryKey, GlossaryItem> = {
  holomorphic_geometry: {
    creative: 'Nitidez Quântica do Som',
    expert: 'Geometria Holomorfa Conforme',
    tooltip: 'Transforma manchas e borrões sonoros em linhas finas e brilhantes calculando a fase exata de cada onda.'
  },
  reassignment: {
    creative: 'Foco Infinito de Harmônicos',
    expert: 'Reatribuição STFT Auger-Flandrin',
    tooltip: 'Como ajustar o anel de foco de uma câmera: concentra a energia espectral exatamente onde as notas estão vibrando.'
  },
  cr_residual: {
    creative: 'Divergência de Fase',
    expert: 'Resíduo de Cauchy-Riemann',
    tooltip: 'Mede o quanto a forma da onda se desvia de uma vibração pura e perfeitamente estável.'
  },
  stn_decomposition: {
    creative: 'Fatiador de Som (Corpo, Estalo e Ar)',
    expert: 'Decomposição Tríplice STN',
    tooltip: 'Separa de forma independente o corpo do som (senóides), a pancada/ataque (transientes) e o chiado/ar (ruído).'
  },
  rdo_dead_zone: {
    creative: 'Economia Máxima de Tamanho',
    expert: 'Quantização Dead-Zone RDO',
    tooltip: 'Descarta detalhes inaudíveis do áudio para encolher o arquivo em até 34x sem degradar a qualidade do timbre.'
  },
  inharmonicity_b: {
    creative: 'Brilho Metálico',
    expert: 'Fator de Rigidez Inarmônica B',
    tooltip: 'Controla a ressonância metálica natural de cordas grossas e tensas (como cordas graves de piano ou barras de xilofone).'
  },
  taylor_jets: {
    creative: 'Escultura de Timbre',
    expert: 'Jatos de Taylor e Campo Escalar',
    tooltip: 'Permite esculpir a textura do som desenhando curvas de nível e relevos contínuos na tela.'
  }
};

/**
 * Traduz um termo técnico para a terminologia apropriada ao modo do usuário.
 */
export function translateTerm(key: GlossaryKey, mode: UiMode): string {
  const item = GLOSSARY_MAP[key];
  if (!item) {
    return key as string;
  }
  return mode === 'creative' ? item.creative : item.expert;
}

/**
 * Retorna uma dica educativa clara e compreensível para qualquer usuário.
 */
export function getEducationalTooltip(key: GlossaryKey): string {
  const item = GLOSSARY_MAP[key];
  if (!item) {
    return 'Controle ajustável de processamento de áudio.';
  }
  return item.tooltip;
}
