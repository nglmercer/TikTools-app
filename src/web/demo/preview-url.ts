import { tourRegistry } from '@/features/guided-help/model/registry.ts';
import { demoParam, demoSearchParams } from './contract.ts';
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
  const params = demoSearchParams({
    [demoParam.scenario]: scenario,
    [demoParam.locale]: locale,
    [demoParam.instance]: previewInstance(id),
    [demoParam.framed]: true,
    [demoParam.preview]: true,
    [demoParam.theme]: document.documentElement.dataset.theme === 'light' ? 'light' : 'dark',
    [demoParam.route]: route,
  });
  const url = new URL(window.location.href);
  url.search = params.toString();
  url.hash = route;
  return url.href;
}
