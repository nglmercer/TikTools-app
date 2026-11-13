import { parseDemoOptions } from './scenarios.ts';
import { installDemoStorage, memoryStorage } from './storage.ts';
import { contextInstance, contextName } from './context-instance.ts';
import './demo.css';

// Storage is configured BEFORE importing modules that cache preferences or journals.
try {
  const options = parseDemoOptions(location.search);
  options.instance = contextInstance(options.instance, window.name, () => crypto.randomUUID());
  window.name = contextName(options.instance);
  const url = new URL(location.href); url.searchParams.set('instance', options.instance);
  history.replaceState(history.state, '', url);
  let backing: { local: Storage; session: Storage };
  try { backing = { local: localStorage, session: sessionStorage }; backing.session.setItem(`tiktools.demo.${options.instance}.probe`, '1'); backing.session.removeItem(`tiktools.demo.${options.instance}.probe`); }
  catch { backing = { local: memoryStorage(), session: memoryStorage() }; }
  const storage = installDemoStorage(options.instance, backing);
  document.documentElement.lang = options.locale;
  const { bootDemo } = await import('./mount-demo.ts');
  await bootDemo(options, storage);
} catch (error) {
  const message = document.createElement('p'); message.setAttribute('role', 'alert');
  message.textContent = error instanceof Error ? error.message : 'Demo unavailable.';
  document.body.replaceChildren(message);
}
