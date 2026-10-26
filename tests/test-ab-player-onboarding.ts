import assert from 'assert';

// Módulo a ser implementado na Green Phase
import {
  computeCompressionMetrics,
  getOnboardingDismissed,
  setOnboardingDismissed
} from '../src/lib/ui/ab_player_logic';

console.log('🧪 Iniciando Teste TDD (Red Phase): Lógica de Player A/B e Estado do Onboarding...');

// 1. Validar métricas de compressão para usuário leigo
const metrics1 = computeCompressionMetrics(475180, 13832); // Exemplo do codec DRED
assert(Math.abs(metrics1.ratio - 34.35) < 0.1, `Razão esperada ~34.4x, obteve ${metrics1.ratio}`);
assert(Math.abs(metrics1.savingsPct - 97.09) < 0.1, `Economia esperada ~97.1%, obteve ${metrics1.savingsPct}`);
assert(metrics1.formattedSavings.includes('34.4x menor'), 'Texto formatado deve conter razão legível');
assert(metrics1.formattedSavings.includes('97.1%'), 'Texto formatado deve conter economia percentual');
console.log(`  ✓ Métricas DRED validadas: "${metrics1.formattedSavings}"`);

// 2. Caso de borda: Sem compressão (Lossless / Mesmo tamanho)
const metricsLossless = computeCompressionMetrics(1000, 1000);
assert.strictEqual(metricsLossless.ratio, 1.0, 'Razão 1:1 para arquivos iguais');
assert.strictEqual(metricsLossless.savingsPct, 0.0, '0% de economia para mesmo tamanho');
console.log('  ✓ Caso de borda 1:1 validado');

// 3. Simulação de LocalStorage para o Onboarding Modal
class MockStorage {
  private data: Record<string, string> = {};
  getItem(k: string) { return this.data[k] ?? null; }
  setItem(k: string, v: string) { this.data[k] = v; }
  removeItem(k: string) { delete this.data[k]; }
}

const mockStorage = new MockStorage();

// Primeira visita: Não deve estar dispensado (deve abrir o modal)
assert.strictEqual(getOnboardingDismissed(mockStorage as any), false, 'Primeira visita deve exibir modal');

// Usuário fecha marcando para não exibir novamente
setOnboardingDismissed(true, mockStorage as any);
assert.strictEqual(getOnboardingDismissed(mockStorage as any), true, 'Visita subsequente deve respeitar dispensa');

console.log('  ✓ Persistência e regras de exibição do modal de onboarding validadas');

console.log('\n🎉 TODOS OS TESTES DE PLAYER A/B E ONBOARDING PASSARAM COM SUCESSO!\n');
