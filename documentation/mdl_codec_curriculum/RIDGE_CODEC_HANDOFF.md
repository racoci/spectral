# Handoff: Codec de Cristas com Dicionário Aprendido & Surprise Encoding

## 1. Visão Geral
Este módulo integra o rastreador de cristas bayesiano (Fase 7) com o pipeline de compressão estrutural (Fase 8), atingindo bitrates inferiores a 6 kbps para áudio polifônico contínuo.

## 2. Mitigação de Riscos de Deriva (Drift Mitigation)
- **Risco**: Erros acumulados em predição autônoma prolongada no decodificador.
- **Solução Implementada**: Intervalo de *heartbeat* obrigatório ($T_{\text{max}} = 15$ quadros $\approx 75\text{ ms}$). Mesmo em ausência de surpresa, um frame síncrono é transmitido periodicamente, zerando qualquer desvio residual de integração.

## 3. Próximos Passos
- Conexão do bitstream de cristas com o módulo `rdo_codec.rs` para empacotamento unificado em contêiner binário `.dred`.
- Integração da visualização das trajetórias reconstruídas com o overlay de cristas no WebGL Editor.
