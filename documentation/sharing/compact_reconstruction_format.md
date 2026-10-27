# Formato Próprio Compacto de Reconstrução de Sessão (`.spectral`)

## 1. Visão Geral
Em vez de trafegar arquivos PCM brutos de áudio (que ocupam centenas de kilobytes), o Spectral adota o paradigma estrutural da teoria MDL: trafegar apenas os parâmetros generativos ontológicos.

Um link de URL de menos de 1 KB é capaz de reconstruir a experiência visual e sintetizar o áudio com fidelidade de estúdio de forma 100% autônoma no navegador receptor.

## 2. Estrutura do Pacote (`SpectralPackage`)
```typescript
export interface SpectralPackage {
  v: 1;
  tool: 'converter' | 'editor' | 'synth' | 'stn' | 'lean' | 'field';
  title?: string;
  params: Record<string, any>;
}
```

## 3. Serialização URL-Safe
- O objeto JSON é minificado (espaços em branco removidos).
- A string UTF-8 é convertida para Base64 URL-safe (substituindo `+` por `-`, `/` por `_` e omitindo `=` finais).
- O payload resultante é incorporado como parâmetro `p` na hash da URL:
  `#/field?p=eyJ2IjoxLCJ0b29sIjoiZmllbGQi...`
