import { computed, ref, watch } from 'vue';
import type { ControlClient } from '@/shared/api/control-client.ts';
import type { ControllerModel } from './controller.ts';

type Account = { state: 'unconfigured' | 'signed-out' | 'signing-in' | 'signed-in'; origin?: string | null; accountId?: string; email?: string; displayName?: string; loginOperationId?: string; error?: string };
type Status = { account: Account; deviceId: string | null };
export interface CloudTeam { id: string; name: string; role: string; clientKind: string }
export type CloudConnectionModel = Pick<ControllerModel, 'status' | 'inventory' | 'refresh' | 'connect' | 'disconnect'>;

export function useControllerCloud(control: ControlClient, model: CloudConnectionModel) {
  const account = ref<Account | null>(null);
  const deviceId = ref<string | null>(null);
  const teams = ref<CloudTeam[]>([]);
  const selected = ref('');
  const profile = ref(''); const label = ref('Moderator'); const token = ref('');
  const busy = ref(false); const error = ref(''); const browserUrl = ref('');
  const signedIn = computed(() => account.value?.state === 'signed-in');
  const signingIn = computed(() => account.value?.state === 'signing-in');
  const setupAllowed = computed(() => ['idle', 'failed'].includes(model.status.value.phase));
  const pending = computed(() => model.status.value.cloudPairing ?? null);
  let stopped = false; let generation = 0; let polling = false;
  let pairTimer: ReturnType<typeof setTimeout> | undefined;
  let accountTimer: ReturnType<typeof setTimeout> | undefined;
  function apply(next: Status): void {
    const identity = (value: Account | null) => JSON.stringify([value?.origin, value?.accountId]);
    if (identity(account.value) !== identity(next.account)) { teams.value = []; selected.value = ''; token.value = ''; }
    account.value = next.account; deviceId.value = next.deviceId;
    if (next.account.state !== 'signing-in') browserUrl.value = '';
  }
  function failure(value: unknown): void { error.value = value instanceof Error ? value.message : 'Cloud unavailable'; }
  async function refreshTeams(): Promise<void> {
    if (!signedIn.value || !deviceId.value || stopped) { teams.value = []; return; }
    const current = generation;
    try {
      const next = await control.call<{ items: CloudTeam[] }>('controller.cloud.teams', {}, { timeoutMs: 25000 });
      if (stopped || generation !== current) return;
      teams.value = next.items;
      if (!teams.value.some((team) => team.id === selected.value)) selected.value = teams.value[0]?.id ?? '';
    } catch (value) { if (!stopped && generation === current) failure(value); }
  }
  async function refreshAccount(): Promise<void> {
    const current = generation;
    try {
      const wasSignedIn = signedIn.value;
      const next = await control.call<Status>('controller.cloud.status');
      if (stopped || current !== generation) return;
      apply(next);
      if (!wasSignedIn && signedIn.value) await refreshTeams();
    } catch (value) { if (!stopped && current === generation) failure(value); }
    if (!stopped && signingIn.value) accountTimer = setTimeout(() => { void refreshAccount(); }, 1500);
  }
  async function start(): Promise<void> {
    stopped = false; busy.value = true;
    try {
      apply(await control.call<Status>('controller.cloud.status'));
      if (stopped) return;
      if (account.value?.state === 'signed-out') apply(await control.call<Status>('controller.cloud.restore', {}, { timeoutMs: 25000 }));
      if (!stopped) { await refreshTeams(); if (signingIn.value) void refreshAccount(); }
    } catch (value) { if (!stopped) failure(value); }
    finally { if (!stopped) busy.value = false; }
  }
  async function login(): Promise<void> {
    if (busy.value || !setupAllowed.value || stopped) return;
    busy.value = true; error.value = '';
    const current = ++generation;
    try {
      const begin = await control.call<{ browserUrl: string; operationId: string }>('controller.cloud.login.begin', {}, { timeoutMs: 25000 });
      if (stopped || current !== generation) return;
      browserUrl.value = begin.browserUrl;
      await refreshAccount();
      try { await control.call('controller.cloud.login.open', { operationId: begin.operationId }); }
      catch (value) { if (!stopped && current === generation) failure(value); }
    } catch (value) { if (!stopped && current === generation) failure(value); }
    finally { if (!stopped && current === generation) busy.value = false; }
  }
  async function openLogin(): Promise<void> {
    const operationId = account.value?.loginOperationId;
    if (!operationId || stopped) return;
    try { await control.call('controller.cloud.login.open', { operationId }); } catch (value) { if (!stopped) failure(value); }
  }
  async function cancelLogin(): Promise<void> {
    const operationId = account.value?.loginOperationId;
    if (!operationId || stopped) return;
    const current = ++generation;
    if (accountTimer) clearTimeout(accountTimer);
    try { const next = await control.call<Status>('controller.cloud.login.cancel', { operationId }); if (!stopped && current === generation) apply(next); }
    catch (value) { if (!stopped && current === generation) { failure(value); await refreshAccount(); } }
  }
  async function logout(): Promise<void> {
    if (busy.value || stopped) return;
    const current = ++generation; busy.value = true; error.value = ''; token.value = '';
    try { const next = await control.call<Status>('controller.cloud.logout'); if (!stopped && current === generation) { apply(next); await model.refresh(); } }
    catch (value) { if (!stopped && current === generation) failure(value); }
    finally { if (!stopped && current === generation) busy.value = false; }
  }
  async function join(): Promise<void> {
    if (busy.value || !signedIn.value || !setupAllowed.value || stopped) return;
    const current = generation; busy.value = true; error.value = '';
    const params = { profile: profile.value, label: label.value, token: token.value }; token.value = '';
    try {
      await control.call('controller.cloud.join', params, { timeoutMs: 60000 });
      if (stopped || current !== generation) return;
      await model.refresh(); await refreshAccount();
    } catch (value) { if (!stopped && current === generation) failure(value); }
    finally { if (!stopped && current === generation) busy.value = false; }
  }
  async function connectTeam(): Promise<void> {
    if (busy.value || !signedIn.value || !setupAllowed.value || !selected.value || stopped) return;
    const current = generation; const epoch = model.status.value.epoch;
    busy.value = true; error.value = '';
    try {
      const result = await control.call<{ profile: string }>('controller.cloud.profile.create', { profile: profile.value, hostId: selected.value }, { timeoutMs: 100000 });
      if (stopped || current !== generation) return;
      await model.inventory(); await model.refresh();
      if (!stopped && current === generation && model.status.value.epoch === epoch) await model.connect(result.profile);
    } catch (value) { if (!stopped && current === generation) failure(value); }
    finally { if (!stopped && current === generation) busy.value = false; }
  }
  async function poll(): Promise<void> {
    if (stopped || polling || model.status.value.phase !== 'cloud-pairing' || !pending.value) return;
    polling = true; const epoch = model.status.value.epoch; const current = generation;
    try {
      const result = await control.call<{ approved: boolean; profile?: string }>('controller.cloud.pair.poll', { epoch }, { timeoutMs: 100000 });
      if (stopped || current !== generation) return;
      if (result.approved && result.profile) await model.inventory();
      await model.refresh();
      if (model.status.value.epoch !== epoch) return;
      if (result.approved && result.profile) { await model.connect(result.profile); await refreshTeams(); }
    } catch (value) { if (!stopped && current === generation) failure(value); }
    finally {
      polling = false;
      if (!stopped && model.status.value.phase === 'cloud-pairing') pairTimer = setTimeout(() => { void poll(); }, 10000);
    }
  }
  async function cancelPairing(): Promise<void> { if (pairTimer) clearTimeout(pairTimer); await model.disconnect(); }
  const unwatch = watch(() => `${model.status.value.phase}:${model.status.value.epoch}`, () => {
    if (pairTimer) clearTimeout(pairTimer);
    if (model.status.value.phase === 'cloud-pairing') pairTimer = setTimeout(() => { void poll(); }, 0);
  }, { immediate: true });
  function stop(): void { stopped = true; generation += 1; token.value = ''; if (pairTimer) clearTimeout(pairTimer); if (accountTimer) clearTimeout(accountTimer); unwatch(); }
  return { account, deviceId, teams, selected, profile, label, token, busy, error, browserUrl, signedIn, signingIn, setupAllowed, pending, start, stop, login, openLogin, cancelLogin, logout, join, connectTeam, refreshTeams, cancelPairing };
}
