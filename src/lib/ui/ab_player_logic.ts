export interface CompressionMetrics {
  originalBytes: number;
  compressedBytes: number;
  ratio: number;
  savingsPct: number;
  formattedSavings: string;
}

/**
 * Calcula métricas de compressão legíveis para usuários comuns.
 */
export function computeCompressionMetrics(
  originalBytes: number,
  compressedBytes: number
): CompressionMetrics {
  if (originalBytes <= 0 || compressedBytes <= 0) {
    return {
      originalBytes,
      compressedBytes,
      ratio: 1.0,
      savingsPct: 0.0,
      formattedSavings: 'Sem redução detectada'
    };
  }

  const ratio = parseFloat((originalBytes / compressedBytes).toFixed(2));
  const savingsPct = parseFloat((Math.max(0, 1.0 - (compressedBytes / originalBytes)) * 100).toFixed(1));

  let formattedSavings = '';
  if (ratio > 1.05) {
    formattedSavings = `Arquivo ${ratio.toFixed(1)}x menor (Economia de ${savingsPct.toFixed(1)}%)`;
  } else {
    formattedSavings = 'Tamanho equivalente (Modo Lossless Bit-Exact)';
  }

  return {
    originalBytes,
    compressedBytes,
    ratio,
    savingsPct,
    formattedSavings
  };
}

const ONBOARDING_STORAGE_KEY = 'spectral_onboarding_dismissed';

/**
 * Verifica se o modal de onboarding já foi dispensado pelo usuário.
 */
export function getOnboardingDismissed(storage: Storage = typeof window !== 'undefined' ? window.localStorage : null as any): boolean {
  if (!storage) return false;
  try {
    return storage.getItem(ONBOARDING_STORAGE_KEY) === 'true';
  } catch {
    return false;
  }
}

/**
 * Define se o modal de onboarding não deve mais ser exibido automaticamente.
 */
export function setOnboardingDismissed(dismissed: boolean, storage: Storage = typeof window !== 'undefined' ? window.localStorage : null as any): void {
  if (!storage) return;
  try {
    storage.setItem(ONBOARDING_STORAGE_KEY, dismissed ? 'true' : 'false');
  } catch {
    // Ignore storage quota/permission errors in restricted environments
  }
}
