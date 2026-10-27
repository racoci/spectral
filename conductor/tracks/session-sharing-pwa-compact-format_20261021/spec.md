# Especificação: Compartilhamento de Sessão por Formato Estrutural Compacto, PWA Offline e Exportação Vetorial

## 1. Visão Geral
Esta track resolve a portabilidade e a autonomia do Spectral. Em vez de transmitir arquivos pesados de áudio PCM (que consomem centenas de kilobytes ou megabytes), criamos um **formato compacto próprio de descrição estrutural** (`.spectral` / payload de URL).

O formato armazena apenas a "partitura matemática" necessária para o motor do Spectral sintetizar o som e renderizar os gráficos correspondentes no cliente destino sem precisar do áudio original. Além disso, adicionamos suporte completo a **Progressive Web App (PWA)** com execução 100% offline e **exportação vetorial em SVG**.

---

## 2. Requisitos Funcionais

### 2.1 Formato Compacto Próprio de Reconstrução (`SpectralPackage`)
- Estrutura de dados canônica serializável:
  ```typescript
  export interface SpectralPackage {
    v: 1; // Versão do schema
    tool: 'converter' | 'editor' | 'synth' | 'stn' | 'lean' | 'field';
    title?: string;
    params: Record<string, any>; // Coeficientes analíticos mínimos
  }
  ```
- **Conteúdo Específico por Ferramenta**:
  - `stn`: Parâmetros do modelo (ganhos S/T/N, frequências e amplitudes de senoides, instantes de transientes, coeficientes de ruído).
  - `lean`: Código Lean 4 puro ou patch analítico DX7 (algoritmo 1..32, 6 operadores, envelopes).
  - `field`: Modelo do campo escalar (centro $p_0$, escala espacial, coeficientes angulares de Taylor $q_n(\theta)$ ordens 0 a 8).
  - `synth`: Parâmetros de formantes acústicos, $f_0$, inarmonicidade $B$ e nós do grafo TreeNN.
- **Ressíntese no Destino**: O navegador que recebe o pacote alimenta diretamente o motor WebAssembly / Web Audio, gerando áudio com fidelidade idêntica a partir de menos de 1 KB de dados.

### 2.2 Compartilhamento por URL com Query String
- O botão `[🔗 Compartilhar]` na barra superior serializa a sessão ativa em URL amigável:
  `#/field?data=...` ou `#/lean?patch=...`
- Copia automaticamente para a área de transferência com notificação visual tipo *toast* ("Link copiado para a área de transferência!").
- Na abertura da aplicação, o roteador detecta a query string no hash e restaura o estado imediatamente.

### 2.3 Suporte Completo a PWA & Service Worker Offline
- Manifesto Web (`public/manifest.webmanifest`) configurado para instalação nativa no Android, iOS, Windows, macOS e Linux.
- Service Worker (`public/sw.js`) com estratégia *Stale-While-Revalidate* e cache prioritário para:
  - Binários WebAssembly (`core_wasm_bg.wasm`).
  - Amostras de demonstração (`demos/*.wav`).
  - Fontes e scripts essenciais da interface.
- Permite abrir e usar todas as 6 ferramentas mesmo sem nenhuma conexão à internet.

### 2.4 Exportação Gráfica Estrutural em SVG
- Módulo gerador de SVG que desenha trajetórias de cristas espectrais, curvas de nível de campos de Taylor e envelopes como vetores `<path>` e `<polygon>` escaláveis em resolução infinita, prontos para ilustrações e design.

---

## 3. Requisitos Não-Funcionais
- **Tamanho do Payload**: $< 2\text{ KB}$ para compatibilidade universal em navegadores e mensageiros.
- **Velocidade de Reconstituição**: $< 30\text{ ms}$ para decodificar e popular os controles no destino.
- **Testes Automatizados**: Baterias completas em `tests/test-session-sharing-compact.ts`.

---

## 4. Critérios de Aceitação
- [x] O formato estrutural serializa e desserializa estados de todas as ferramentas com 100% de integridade.
- [x] O botão `Compartilhar` gera link funcional testável em aba anônima.
- [x] O arquivo `manifest.webmanifest` e o Service Worker são válidos e registram no navegador.
- [x] O gerador de SVG produz arquivos vetoriais válidos e visualizáveis.
- [x] `npm run check` e `npm run test:integration` passam com 100% de sucesso.

---

## 5. Análise Recursiva de Riscos e Mitigações (6 Níveis de Profundidade)

*   **[Profundidade 1] Risco:** URLs muito extensas serem truncadas por aplicativos de mensagem ou navegadores legados (Probabilidade Média).
    *   **Mitigação:** Codificar apenas parâmetros delta e omitir valores que coincidem com os padrões do preset (omit-defaults), mantendo o tamanho sob 1.5 KB.
        *   **[Profundidade 2] Risco:** Novos campos no futuro quebrarem links gerados por versões antigas (Probabilidade Média).
            *   **Mitigação:** Incluir campo de versão estrito `v: 1` no pacote com preenchimento gracioso de propriedades ausentes por defaults seguros.
                *   **[Profundidade 3] Risco:** Injeção de código malicioso através de strings manipuladas na URL (Probabilidade Baixa) -> *Ramo Encerrado (Baixa Probabilidade).*
*   **[Profundidade 1] Risco:** Service Worker reter versões desatualizadas do binário WebAssembly após novo deploy no GitHub Pages (Probabilidade Média).
    *   **Mitigação:** Versionar o nome do cache no Service Worker (`CACHE_VERSION = 'v1.1'`) e disparar limpeza imediata de caches antigos no evento `activate`.
        *   **[Profundidade 2] Risco:** Falha de rede durante atualização inicial do cache (Probabilidade Baixa) -> *Ramo Encerrado (Baixa Probabilidade).*
