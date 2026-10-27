export type SpectralToolId = 'converter' | 'editor' | 'synth' | 'stn' | 'lean' | 'field';

export interface SpectralPackage {
  v: 1;
  tool: SpectralToolId;
  title?: string;
  params: Record<string, any>;
}

/**
 * Codifica uma string UTF-8 em Base64 URL-safe compatível com browser e Node.js.
 */
function toBase64Url(str: string): string {
  if (typeof Buffer !== 'undefined') {
    return Buffer.from(str, 'utf-8').toString('base64url');
  }
  // Browser fallback
  const base64 = window.btoa(unescape(encodeURIComponent(str)));
  return base64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

/**
 * Decodifica Base64 URL-safe para string UTF-8 compatível com browser e Node.js.
 */
function fromBase64Url(base64url: string): string {
  if (typeof Buffer !== 'undefined') {
    return Buffer.from(base64url, 'base64url').toString('utf-8');
  }
  // Browser fallback
  let base64 = base64url.replace(/-/g, '+').replace(/_/g, '/');
  while (base64.length % 4) {
    base64 += '=';
  }
  return decodeURIComponent(escape(window.atob(base64)));
}

/**
 * Serializa um pacote SpectralPackage em string compacta URL-safe.
 */
export function encodeSpectralPackage(pkg: SpectralPackage): string {
  const jsonStr = JSON.stringify(pkg);
  return toBase64Url(jsonStr);
}

/**
 * Desserializa uma string compacta em SpectralPackage com validação de esquema.
 */
export function decodeSpectralPackage(encoded: string): SpectralPackage | null {
  try {
    const jsonStr = fromBase64Url(encoded);
    const parsed = JSON.parse(jsonStr);
    if (!parsed || parsed.v !== 1 || !parsed.tool || !parsed.params) {
      return null;
    }
    return parsed as SpectralPackage;
  } catch {
    return null;
  }
}

/**
 * Gera a rota de hash com query string correspondente ao pacote (ex: #/stn?p=...).
 */
export function serializeToHashQuery(pkg: SpectralPackage): string {
  const encoded = encodeSpectralPackage(pkg);
  return `#/${pkg.tool}?p=${encoded}`;
}

/**
 * Extrai e desserializa o pacote a partir de uma hash de URL (ex: #/stn?p=...).
 */
export function parseFromHashQuery(hash: string): SpectralPackage | null {
  if (!hash || !hash.includes('?')) {
    return null;
  }

  const queryPart = hash.substring(hash.indexOf('?') + 1);
  const params = new URLSearchParams(queryPart);
  const encodedPayload = params.get('p');

  if (!encodedPayload) {
    return null;
  }

  return decodeSpectralPackage(encodedPayload);
}
