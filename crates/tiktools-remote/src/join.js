(() => {
  const invitation = new URLSearchParams(location.hash.slice(1));
  history.replaceState(null, '', location.pathname);
  const status = document.getElementById('status');
  const form = document.getElementById('join');
  const inviteId = invitation.get('inviteId');
  const secret = invitation.get('secret');
  invitation.delete('secret');
  if (!inviteId || !secret) {
    status.textContent = 'Open the invitation link shared by your host.';
    form.hidden = true;
    return;
  }
  const encodeHex = (bytes) => Array.from(bytes, (value) => value.toString(16).padStart(2, '0')).join('');
  async function post(path, input) {
    const response = await fetch(path, { method: 'POST', credentials: 'same-origin', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(input) });
    const value = await response.json();
    if (!response.ok && value.error?.code !== 'pairing_pending') throw new Error(value.error?.code ?? 'connection_failed');
    return value;
  }
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    const button = form.querySelector('button');
    button.disabled = true;
    try {
      const exchangeSecret = encodeHex(crypto.getRandomValues(new Uint8Array(32)));
      const exchangeSecretHash = encodeHex(new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(exchangeSecret))));
      const pending = await post('/remote/v1/pair/redeem', { inviteId, secret, clientLabel: document.getElementById('label').value, clientKind: 'browser', exchangeSecretHash });
      status.textContent = 'Waiting for your host to approve this device…';
      while (Date.now() < pending.expiresAt) {
        await new Promise((resolve) => setTimeout(resolve, 1500));
        const result = await post('/remote/v1/pair/status', { requestId: pending.requestId, exchangeSecret });
        if (result.approved) {
          status.textContent = 'Device approved. You can close this page.';
          form.hidden = true;
          return;
        }
      }
      throw new Error('pairing_expired');
    } catch {
      status.textContent = 'Unable to join. Ask your host for a new invitation.';
    }
  });
})();
