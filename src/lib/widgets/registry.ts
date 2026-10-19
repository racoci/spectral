import type { Component } from 'svelte';
import type { WidgetKind, WidgetDescriptor } from './types';

export type WidgetComponentProps = {
  descriptor: WidgetDescriptor;
  onParamChange?: (paramPath: string, newValue: number | string | boolean) => void;
};

// Em ambiente de teste unitário Node.js (tsx direto sem Vite bundler),
// utilizamos referências stub válidas se a extensão .svelte não for suportada nativamente pelo Node.
let OperatorMatrixComponent: any = null;
let EnvelopeStageComponent: any = null;
let ParameterKnobComponent: any = null;

try {
  OperatorMatrixComponent = (await import('./OperatorMatrixWidget.svelte')).default;
  EnvelopeStageComponent = (await import('./EnvelopeStageWidget.svelte')).default;
  ParameterKnobComponent = (await import('./ParameterKnobWidget.svelte')).default;
} catch {
  OperatorMatrixComponent = { name: 'OperatorMatrixWidget', __isComponentStub: true };
  EnvelopeStageComponent = { name: 'EnvelopeStageWidget', __isComponentStub: true };
  ParameterKnobComponent = { name: 'ParameterKnobWidget', __isComponentStub: true };
}

export const widgetRegistry: Partial<Record<WidgetKind, any>> = {
  'fm-algorithm': OperatorMatrixComponent,
  'envelope-stage': EnvelopeStageComponent,
  'parameter-knob': ParameterKnobComponent
};

/**
 * Retorna o componente Svelte 5 registrado para uma categoria semântica de widget.
 */
export function getWidgetComponent(kind: WidgetKind): any {
  return widgetRegistry[kind] || null;
}
