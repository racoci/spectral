# Especificação: UI/UX Acessibilidade, Demos de 1 Clique e Modo Criativo vs Expert

## 1. Visão Geral
Esta track tem como objetivo democratizar o acesso e o uso prático de todas as inovações do projeto (geometria holomorfa, síntese aditiva DDSP, decomposição STN, IDE de campos escalares e compressão RDO).

Transformamos a experiência do usuário leigo, músico ou designer de som através de:
1. **Demos Rápidas de 1 Clique**: Carregamento imediato de amostras curadas sem necessidade de upload de arquivos próprios.
2. **Alternador Global de Modo [Criativo 🎨 / Expert 🔬]**: Camada de tradução de termos complexos para conceitos sonoros intuitivos (ex: "Foco Infinito", "Corpo, Estalo e Ar", "Economia de Tamanho").
3. **Tooltips Educativos Inteligentes**: Explicações contextuais em linguagem clara sobre o impacto perceptual de cada ajuste.
4. **Player A/B Comparativo**: Audição instantânea do áudio original versus processado/comprimido com métricas compreensíveis de economia de bytes.
5. **Modal de Boas-Vindas e Atalhos Rápidos**: Apresentação inicial clara das 6 ferramentas com guia de início rápido.

---

## 2. Requisitos Funcionais

### 2.1 Barra de Demonstrações Rápidas de 1 Clique (1-Click Audio Demos)
- Disponibilizar na barra de navegação global (`GlobalTopbar.svelte`) ou no topo do editor um seletor rápido com 3 amostras canônicas:
  - 🗣️ **Voz Falada** (`voice.wav`)
  - 🎹 **Sintetizador** (`synth.wav`)
  - 🎺 **Harmônicos Ricos** (`car-horn.wav`)
- O clique em uma amostra busca o arquivo via `fetch`, decodifica o buffer de áudio na Web Audio API, inicializa o espectrograma correspondente no WebGL Editor e ativa o player de áudio com reprodução disponível imediatamente.

### 2.2 Alternador Global de Modo [Criativo 🎨 / Expert 🔬]
- Toggle na barra superior com estado persistido no `localStorage`:
  - **Modo Criativo (Padrão para novos usuários)**:
    - Rótulos simplificados e metáforas musicais:
      - *Geometria Holomorfa* $\rightarrow$ *Nitidez Quântica*
      - *Reatribuição Auger-Flandrin* $\rightarrow$ *Foco Infinito*
      - *Resíduo de Cauchy-Riemann* $\rightarrow$ *Divergência de Fase*
      - *Decomposição STN* $\rightarrow$ *Fatiador de Som (Corpo, Estalo e Ar)*
      - *Rigidez Inarmônica B* $\rightarrow$ *Brilho Metálico*
      - *Dead-zone RDO* $\rightarrow$ *Economia Máxima de Tamanho*
      - *Jato de Taylor e Campo Escalar* $\rightarrow$ *Escultura de Timbre*
    - Oculta controles excessivamente técnicos secundários (ex: ordem de derivadas acima de 4) para evitar sobrecarga cognitiva.
  - **Modo Expert**:
    - Exibe a terminologia matemática rigorosa completa, fórmulas, ordens de derivadas de 0 a 8 e valores em Hertz/rad/cents.

### 2.3 Tooltips Educativos Inteligentes
- Integrar o componente `SmartTooltip.svelte` nos principais controles de todas as 6 ferramentas com textos descritivos curtos explicando o efeito auditivo de cada slider/knob.

### 2.4 Player A/B Comparativo
- Painel compacto com botões de alternância síncrona:
  - `[▶ Ouvir Original]` e `[▶ Ouvir Comprimido / Processado]`
  - Indicador leigo: *"Arquivo 34x menor (Economia de 97.1% de espaço) • Qualidade perceptual: 98%"*.
  - Botão de download rápido: `[💾 Baixar Áudio WAV]` e `[🖼️ Baixar Imagem PNG]`.

### 2.5 Modal de Boas-Vindas e Tour Inicial (Onboarding)
- Modal acessível exibido na primeira visita (com opção de não mostrar novamente):
  - Etapa 1: Apresentação das 6 ferramentas e seus propósitos.
  - Etapa 2: Como carregar ou tocar um som de demonstração em 1 clique.
  - Etapa 3: Teclas de atalho para navegação rápida (`Alt + 1` a `Alt + 6`).

---

## 3. Requisitos Não-Funcionais
- **Performance**: Carregamento assíncrono dos áudios de demonstração sem travar a thread principal.
- **Acessibilidade**: Rótulos com atributos ARIA (`aria-label`, `role="dialog"` para o modal).
- **Compatibilidade**: Zero regressões nas 25 suítes de teste de integração existentes.

---

## 4. Critérios de Aceitação
- [x] O usuário consegue ouvir e visualizar um áudio de demonstração em menos de 2 cliques após abrir a página.
- [x] O toggle `[Criativo / Expert]` atualiza reativamente os textos de botões, abas e painéis.
- [x] O modal de boas-vindas fecha suavemente com `ESC` ou clique fora, salvando o estado no navegador.
- [x] O Player A/B permite comutação instantânea entre áudio de referência e áudio transformado.
- [x] `npm run check` e `npm run test:integration` passam com 100% de sucesso.

---

## 5. Análise Recursiva de Riscos e Mitigações (6 Níveis de Profundidade)

*   **[Profundidade 1] Risco:** Carregamento de arquivos `.wav` de demonstração falhar no GitHub Pages devido a caminhos relativos ou CORS (Probabilidade Média).
    *   **Mitigação:** Utilizar URLs resolvidas via `import.meta.env.BASE_URL` garantindo que os caminhos públicos (`./voice.wav`, etc.) funcionem em qualquer subdiretório de hospedagem estática.
        *   **[Profundidade 2] Risco:** Arquivos de áudio muito pesados consumirem banda excessiva de dados móveis (Probabilidade Média).
            *   **Mitigação:** Carregar o áudio apenas sob demanda (lazy fetch) quando o usuário clicar no botão da demonstração, nunca no carregamento inicial da página.
                *   **[Profundidade 3] Risco:** Usuário offline ou com conexão lenta ver botão desativado sem feedback visual (Probabilidade Baixa) -> *Ramo Encerrado (Baixa Probabilidade).*
*   **[Profundidade 1] Risco:** A simplificação dos termos no Modo Criativo causar confusão em usuários pesquisadores avançados que esperam ver terminologia de DSP (Probabilidade Média).
    *   **Mitigação:** Implementar o toggle no topo de fácil acesso com ícone claro `[🎨 Criativo | 🔬 Expert]`, permitindo comutação instantânea com um único clique.
        *   **[Profundidade 2] Risco:** Perda da preferência selecionada após recarregar a página (Probabilidade Média).
            *   **Mitigação:** Salvar a preferência imediatamente em `localStorage.getItem('spectral_user_mode')`.
                *   **[Profundidade 3] Risco:** LocalStorage desabilitado em modo de navegação anônima (Probabilidade Baixa) -> *Ramo Encerrado (Baixa Probabilidade).*
