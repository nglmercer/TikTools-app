import { enableAutoUnmount, mount } from '@vue/test-utils';
import { afterEach, expect, test } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import CloudOverlay from '../../../../widgets/cloud-overlay/CloudOverlay.vue';
import { addElement, emptyOverlay, readOverlayDocument, updateElement, type OverlayDocument } from '../../../../shared/cloud-overlay-schema.ts';
import CloudOverlayPreview from './CloudOverlayPreview.vue';

enableAutoUnmount(afterEach);

function layoutDocument(): OverlayDocument {
  let document = addElement(emptyOverlay('gift', 'Parity'), 'text');
  document = addElement(document, 'avatar');
  document = updateElement(document, 'text-1', { x: 480, y: 270, width: 960, height: 540, opacity: 0.5 });
  document = updateElement(document, 'avatar-1', { x: 1440, y: 0, width: 192, height: 108 });
  return document;
}

function boxStyles(wrapper: ReturnType<typeof mount>, selector: string): string[] {
  return wrapper.findAll(selector).map((box) => {
    const style = (box.element as HTMLElement).style;
    return [style.left, style.top, style.width, style.height, style.opacity].join('|');
  });
}

test('one valid document gets the same geometry in the editor preview and the OBS widget', () => {
  const document = layoutDocument();
  expect(readOverlayDocument(document)).not.toBeNull();
  const preview = mountWithPlugins(CloudOverlayPreview, { props: { document } });
  const widget = mount(CloudOverlay, { props: { document, values: { 'viewer.name': 'Ana' } } });

  const fromPreview = boxStyles(preview, '.overlay-preview__box');
  expect(fromPreview).toEqual(['25%|25%|50%|50%|0.5', '75%|0%|10%|10%|1']);
  expect(boxStyles(widget, '.cloud-overlay__box')).toEqual(fromPreview);
  expect((preview.find('.overlay-preview').element as HTMLElement).style.aspectRatio)
    .toBe((widget.find('.cloud-overlay').element as HTMLElement).style.aspectRatio);
});

test('presentation stays renderer specific while the geometry is shared', () => {
  const document = addElement(emptyOverlay('chat', 'Presentation'), 'text');
  const field = updateElement(document, 'text-1', {});
  (field.elements[0] as { source: unknown }).source = { kind: 'field', field: 'chat.message' };
  const preview = mountWithPlugins(CloudOverlayPreview, { props: { document: field } });
  const widget = mount(CloudOverlay, { props: { document: field, values: { 'chat.message': 'hello' } } });
  expect(preview.text()).toBe('{chat.message}');
  expect(widget.text()).toBe('hello');
});

test('the OBS widget renders nothing for a null document', () => {
  const widget = mount(CloudOverlay, { props: { document: null, values: {} } });
  expect(widget.find('.cloud-overlay').exists()).toBe(false);
});
