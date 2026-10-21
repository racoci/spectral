export interface NavigationRoute {
  id: 'converter' | 'editor' | 'synth' | 'stn' | 'lean' | 'field';
  hash: string;
  label: string;
  shortcut: string;
  keyNum: string;
  iconSvg: string;
}

export const TOOL_ROUTES: NavigationRoute[] = [
  {
    id: 'converter',
    hash: '#/converter',
    label: '1. Converter',
    shortcut: 'Alt+1',
    keyNum: '1',
    iconSvg: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M7 16V4M7 4L3 8M7 4L11 8M17 8V20M17 20L21 16M17 20L13 16"/></svg>`
  },
  {
    id: 'editor',
    hash: '#/editor',
    label: '2. WebGL Editor',
    shortcut: 'Alt+2',
    keyNum: '2',
    iconSvg: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 9H21M9 21V9"/></svg>`
  },
  {
    id: 'synth',
    hash: '#/synth',
    label: '3. DDSP Studio',
    shortcut: 'Alt+3',
    keyNum: '3',
    iconSvg: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M2 12C2 12 5 4 8 4C11 4 13 20 16 20C19 20 22 12 22 12"/></svg>`
  },
  {
    id: 'stn',
    hash: '#/stn',
    label: '4. STN Synth',
    shortcut: 'Alt+4',
    keyNum: '4',
    iconSvg: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3"/></svg>`
  },
  {
    id: 'lean',
    hash: '#/lean',
    label: '5. Lean IDE',
    shortcut: 'Alt+5',
    keyNum: '5',
    iconSvg: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 3v18l12-9z"/></svg>`
  },
  {
    id: 'field',
    hash: '#/field',
    label: '6. Field IDE',
    shortcut: 'Alt+6',
    keyNum: '6',
    iconSvg: `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 4h16l-8 16z"/></svg>`
  }
];

export function resolveRouteFromShortcut(event: {
  altKey: boolean;
  key: string;
  targetTagName?: string;
  targetClassName?: string;
}): NavigationRoute | null {
  if (!event.altKey) return null;

  const tag = event.targetTagName?.toUpperCase() || '';
  const cls = event.targetClassName || '';

  // Bloqueia em campos de texto
  if (tag === 'INPUT' || tag === 'TEXTAREA' || cls.includes('cm-content')) {
    return null;
  }

  const route = TOOL_ROUTES.find((r) => r.keyNum === event.key);
  return route || null;
}
