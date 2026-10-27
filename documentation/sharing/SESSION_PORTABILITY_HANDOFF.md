# Handoff de Portabilidade, Compartilhamento e PWA

## 1. Visão Geral
Este módulo garante que o Spectral opere como um aplicativo autônomo, instalável e infinitamente compartilhável, sem necessidade de servidores de banco de dados pesados para armazenar sessões de usuários.

## 2. Invariantes de Portabilidade
1. **Sem Servidor de Estado**: Toda a inteligência da sessão reside no pacote compacto `SpectralPackage`. Links compartilhados não dependem de URLs de terceiros ou APIs de backend.
2. **Execução 100% Offline**: O Service Worker (`sw.js`) garante que o aplicativo possa ser aberto em um avião ou estúdio acústico desconectado e executar todas as transformações WebAssembly.
3. **Exportação Vetorial SVG**: Disponibiliza arquivos gráficos vetoriais puros para designers e publicações científicas.
