import type { VNode } from 'vue';
import type { DomainEventEnvelope } from '../shared/event-types.ts';
import type { GatewayStatus } from '../shared/gateway-client.ts';
import type { WidgetClock } from '../shared/clock.ts';

export interface WidgetController {
  handleEnvelope(envelope: DomainEventEnvelope): unknown;
  clear(): void;
  dispose(): void;
}

/** The editor and OBS instantiate the same template, with independent state. */
export interface WidgetTemplate {
  id: string;
  create(search: string, clock?: WidgetClock): {
    controller: WidgetController;
    render(status: GatewayStatus, debug: boolean): VNode;
  };
  samples(): DomainEventEnvelope[];
}

export function defineWidget(template: WidgetTemplate): WidgetTemplate {
  return template;
}

/** JSON-serializable styling contract for an editor; no HTML or scripts. */
export interface WidgetStyle {
  background?: string;
  textColor?: string;
  accent?: string;
  radius?: number;
}

export const widgetStyleFields = [
  { key: 'background', type: 'color', label: 'Background' },
  { key: 'textColor', type: 'color', label: 'Text color' },
  { key: 'accent', type: 'color', label: 'Accent' },
  { key: 'radius', type: 'number', label: 'Corner radius', min: 0, max: 48 },
] as const;

export function styleVariables(style: WidgetStyle = {}): Record<string, string> {
  const variables: Record<string, string> = {};
  for (const key of ['background', 'textColor', 'accent'] as const) {
    const value = style[key];
    if (typeof value === 'string' && /^#[\da-f]{6}([\da-f]{2})?$/i.test(value)) {
      variables[`--widget-${key}`] = value;
    }
  }
  if (typeof style.radius === 'number' && Number.isFinite(style.radius)) {
    variables['--widget-radius'] = `${Math.max(0, Math.min(48, style.radius))}px`;
  }
  return variables;
}
