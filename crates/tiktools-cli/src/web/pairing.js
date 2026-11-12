(() => {
  const form = document.getElementById('pair');
  const input = document.getElementById('code');
  const result = document.getElementById('result');
  const renew = document.getElementById('renew');
  let retryTimer;
  const show = (text, kind = 'error') => { result.textContent = text; result.dataset.kind = kind; };
  const cooldown = seconds => {
    clearTimeout(retryTimer);
    renew.disabled = true;
    retryTimer = setTimeout(() => { renew.disabled = false; }, Math.min(300, Math.max(1, seconds)) * 1000);
  };
  const submitCode = async code => {
    input.value = '';
    try {
      const response = await fetch('/__local/pair', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ code }) });
      if (response.ok) { location.replace('/'); return; }
      show('Incorrect or expired code. Try again or request a new code.');
    } catch { show('Adapter unavailable. Check the terminal where the adapter is running.'); }
  };
  form.addEventListener('submit', event => {
    event.preventDefault();
    submitCode(input.value);
  });
  // One-click terminal links carry the one-use code in the URL fragment, which
  // browsers never send to the server. Clear it immediately, then pair once.
  const auto = (location.hash || '').match(/#pair=([0-9]{6})(?:&|$)/);
  if (auto) {
    try { history.replaceState(null, '', location.pathname); } catch {}
    show('Signing in with the terminal link…', 'info');
    submitCode(auto[1]);
  }
  renew.addEventListener('click', async () => {
    renew.disabled = true; input.value = '';
    show('Requesting a new code…', 'info');
    try {
      const response = await fetch('/__local/pair/renew', { method: 'POST' });
      const body = await response.json();
      if (response.ok) {
        show('New code printed in the terminal. Enter it here within five minutes.', 'success');
        cooldown(30);
      } else if (body.error === 'cooldown') {
        const seconds = Number(body.retryAfterSeconds) || 30;
        show(`Please wait ${seconds} seconds before requesting another code.`, 'info');
        cooldown(seconds);
      } else if (body.error === 'pairing_locked') {
        show('Too many attempts. Restart the adapter to pair again.');
      } else if (body.error === 'already_paired') {
        show('Already paired. Reload this page, or restart the adapter to pair another browser.');
      } else {
        renew.disabled = false; show('Could not request a code. Check the terminal where the adapter is running.');
      }
    } catch { renew.disabled = false; show('Adapter unavailable. Check the terminal where the adapter is running.'); }
  });
})();