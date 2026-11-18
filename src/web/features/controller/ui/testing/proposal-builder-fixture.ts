import { mount, flushPromises } from '@vue/test-utils';
import { expect, it, vi } from 'vitest';
import { ref } from 'vue';
import { i18n, activateLocale } from '@/shared/i18n/i18n.ts';
import { testMessage } from '@/shared/i18n/test-message.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { createProposalModel } from '../../model/proposals.ts';
import { createAutomationModel, type AutomationRow } from '../../model/automations.ts';
import type { ControllerStatus } from '../../model/controller.ts';
import ControllerProposalBuilder from '../ControllerProposalBuilder.vue';

export { expect, it, vi, mount, flushPromises, ref, i18n, activateLocale, testMessage,
  createFakeControlClient, createProposalModel, createAutomationModel, ControllerProposalBuilder,
  type ControllerStatus, type AutomationRow };

export const t = (key: string): string => testMessage('en', key);

export async function setup(kind: 'action' | 'event') {
  await activateLocale('en');
  const status = ref<ControllerStatus>({ proposalsSupported: true, epoch: '1', viewRevision: '1', phase: 'ready', profile: 'desk', origin: 'https://host.example', errorCode: null,
    hello: { connectionId: 'connection', hostId: 'host', hostInstanceId: 'instance', principalId: 'author', sessionId: 'device', grantRevision: '1', capabilities: ['automation.read', 'automation.edit'], features: { automationDrafts: true } } });
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'controller.proposals.list') return { epoch: '1', connectionId: 'connection', hostInstanceId: 'instance', grantRevision: '1', storeRevision: '0', rows: [], next: null };
    if (method === 'controller.status') return JSON.parse(JSON.stringify(status.value));
    if (method === 'controller.automations') return { epoch: '1', automationRevision: '1', rows: [
      { summary: { kind: 'action', id: 'alert.action', name: 'Alert <unsafe>', enabled: true, definitionRedacted: true }, version: { kind: 'automation', scope: 'action', id: 'alert.action', revision: '2' } },
      { summary: { kind: 'event', id: 'other.event', name: 'Other event', enabled: true, definitionRedacted: true }, version: { kind: 'automation', scope: 'event', id: 'other.event', revision: '1' } },
    ], next: null };
    throw new Error(`unexpected ${method}`);
  });
  const send = vi.fn(async () => true);
  const model = createProposalModel(client, { status, blocked: () => false, journalMatches: () => true, send });
  await model.refresh(); model.select(kind, 'author.example');
  const wrapper = mount(ControllerProposalBuilder, { props: { model }, global: { plugins: [i18n] } });
  const button = (label: string) => wrapper.findAll('button').find((node) => node.text() === label)!;
  const dialogButton = (label: string) => wrapper.get('dialog[open]').findAll('button').find((node) => node.text() === label)!;
  return { status, model, wrapper, button, dialogButton, calls, send, client };
}
