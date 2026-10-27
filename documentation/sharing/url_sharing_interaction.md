# Fluxo de Compartilhamento por URL Hash e Restauração de Sessão

## 1. Visão Geral
O fluxo de compartilhamento do Spectral permite que qualquer usuário envie suas criações (paisagens de Taylor, timbres DX7, separação STN ou análises espectrais) através de um link único e compacto.

## 2. Fluxo do Usuário (Compartilhar)
1. O usuário ajusta os parâmetros em qualquer uma das ferramentas.
2. Clica no botão `[🔗 Compartilhar]` na barra superior (`GlobalTopbar`).
3. O software empacota o modelo em um `SpectralPackage` serializado em Base64 URL-safe.
4. O link gerado (ex: `https://.../#/field?p=eyJ2Ijox...`) é copiado automaticamente para a área de transferência.
5. Uma notificação visual (*toast*) discreta surge no canto inferior direito informando: *"Link de compartilhamento copiado! 📋"*.

## 3. Fluxo do Destinatário (Ressíntese no Destino)
1. O destinatário clica no link compartilhado.
2. O roteador do Spectral intercepta o parâmetro `?p=` na hash da URL antes de renderizar.
3. O pacote é validado e aplicado diretamente no estado reativo da ferramenta selecionada.
4. O áudio e os gráficos correspondentes são reconstruídos sem necessidade de download de arquivos pesados de áudio PCM.
