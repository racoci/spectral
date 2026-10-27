# Arquitetura PWA Offline e Cache Stale-While-Revalidate

## 1. Visão Geral
Para permitir o uso do Spectral em qualquer contexto (como palcos, estúdios isolados ou viagens sem conectividade), o aplicativo é configurado como um **Progressive Web App (PWA)** com suporte de execução offline nativo.

## 2. Estratégia de Cache do Service Worker (`sw.js`)
- **Precache na Instalação**:
  - `index.html`, manifesto, ícones SVG e as 3 amostras canônicas em `demos/*.wav`.
- **Estratégia em Tempo de Execução (`fetch`)**:
  - **Stale-While-Revalidate**: Responde imediatamente a partir do cache local enquanto busca atualizações em segundo plano.
  - Binários WebAssembly (`core_wasm_bg.wasm`) e scripts JS são servidos instantaneamente sem latência de rede.
- **Limpeza no Ativamento (`activate`)**:
  - Caches legados com nomes obsoletos são purgados automaticamente ao detectar incremento de `CACHE_NAME`.

## 3. Instalação como Aplicativo Nativo
- Compatível com o menu "Instalar Aplicativo" do Google Chrome, Microsoft Edge e "Adicionar à Tela de Início" no Safari/iOS.
