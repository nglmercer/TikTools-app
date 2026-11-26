import { tourRegistry } from '@/features/guided-help/model/registry.ts';
import type { DemoLocale } from './scenarios.ts';
import { demoTourScenarios, isDemoTour } from './tours.ts';

const previewInstances = new Map<string, string>();
function previewInstance(id: string): string {
  const existing = previewInstances.get(id);
  if (existing) return existing;
  const nonce = crypto.randomUUID().replaceAll('-', '').slice(0, 12);
  const instance = `catalog-${id}-${nonce}`.slice(0, 64);
  previewInstances.set(id, instance);
  return instance;
}

export function demoPreviewUrl(id: string, locale: DemoLocale): string {
  if (!isDemoTour(id)) return '';
  const scenario = demoTourScenarios[id];
  const route = tourRegistry[id].route;
  const params = new URLSearchParams({
    scenario,
    locale,
    instance: previewInstance(id),
    framed: '1',
    preview: '1',
    theme: document.documentElement.dataset.theme === 'light' ? 'light' : 'dark',
    route,
  });
  const url = new URL(window.location.href);
  url.search = params.toString();
  url.hash = route;
  return url.href;
}
