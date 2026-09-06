import { afterEach, describe, expect, test } from 'bun:test';

import { completeDialog, dialogQueue, useDialogs } from './useDialogs.ts';

afterEach(() => {
  while (dialogQueue.length > 0) {
    const request = dialogQueue[0];
    if (!request) break;
    completeDialog(request, request.kind === 'confirm' ? false : request.kind === 'prompt' ? null : undefined);
  }
});

describe('application dialogs', () => {
  test('resolves confirm and prompt requests asynchronously in queue order', async () => {
    const dialogs = useDialogs();
    const confirmed = dialogs.confirm('Delete this?');
    const answer = dialogs.prompt('What is your name?', { initialValue: 'Ada' });

    expect(dialogQueue).toHaveLength(2);
    expect(dialogQueue[0]?.kind).toBe('confirm');

    const firstRequest = dialogQueue[0];
    if (!firstRequest) throw new Error('confirm request was not queued');
    completeDialog(firstRequest, true);
    expect(await confirmed).toBe(true);

    expect(dialogQueue[0]?.kind).toBe('prompt');
    const secondRequest = dialogQueue[0];
    if (!secondRequest) throw new Error('prompt request was not queued');
    completeDialog(secondRequest, 'Grace');
    expect(await answer).toBe('Grace');
  });

  test('alert closes without a native blocking call', async () => {
    const dialogs = useDialogs();
    const acknowledged = dialogs.alert('Saved.');
    const request = dialogQueue[0];
    if (!request) throw new Error('alert request was not queued');

    completeDialog(request, undefined);
    await acknowledged;
    expect(dialogQueue).toHaveLength(0);
  });
});
