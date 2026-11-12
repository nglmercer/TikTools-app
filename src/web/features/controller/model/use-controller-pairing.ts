import { ref } from 'vue';
import type { ControlClient } from '@/shared/api/control-client.ts';
import type { ControllerModel } from './controller.ts';

export function useControllerPairing(control: ControlClient, model: Pick<ControllerModel, 'error' | 'inventory' | 'connect' | 'disconnect'>) {
  const error = model.error;
  const form = ref({ profile: '', origin: '', label: 'Moderator', invitation: '', fingerprint: '', confirmPin: false });
  const fingerprint = ref('');
  const pending = ref('');
  const pairing = ref(false);
  let stopped = false;
  let pollTimer: ReturnType<typeof setTimeout> | undefined;
  async function probe(): Promise<void> {
    try { fingerprint.value = (await control.call<{ fingerprint: string }>('controller.probe', { origin: form.value.origin })).fingerprint; }
    catch (failure) { error.value = failure instanceof Error ? failure.message : 'Trust check failed'; }
  }
  async function poll(): Promise<void> {
    try {
      const result = await control.call<{ approved: boolean; profile?: string }>('controller.pair.poll');
      if (stopped) return;
      if (result.approved && result.profile) { pairing.value = false; pending.value = ''; await model.inventory(); await model.connect(result.profile); }
      else pollTimer = setTimeout(() => { void poll(); }, 1500);
    } catch (failure) { pairing.value = false; error.value = failure instanceof Error ? failure.message : 'Pairing failed'; }
  }
  async function pair(): Promise<void> {
    pairing.value = true; error.value = '';
    const params = { ...form.value, fingerprint: form.value.fingerprint.trim() || null };
    form.value.invitation = '';
    try {
      const reply = await control.call<{ requestId: string }>('controller.pair', params);
      if (stopped) return;
      pending.value = reply.requestId; await poll();
    } catch (failure) { pairing.value = false; error.value = failure instanceof Error ? failure.message : 'Pairing failed'; }
  }
  async function disconnect(): Promise<void> { if (pollTimer) clearTimeout(pollTimer); pending.value = ''; pairing.value = false; await model.disconnect(); }

  function resume(requestId: string): void {
    if (stopped) return;
    pending.value = requestId; pairing.value = true; void poll();
  }
  function stop(): void { stopped = true; if (pollTimer) clearTimeout(pollTimer); }
  return { form, fingerprint, pending, pairing, probe, pair, disconnect, resume, stop };
}
export type ControllerPairing = ReturnType<typeof useControllerPairing>;
