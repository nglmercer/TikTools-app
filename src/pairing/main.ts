import { mountPairing } from './pairing';

mountPairing(document, {
  fetch: (input, init) => fetch(input, init),
  languages: navigator.languages,
  hash: location.hash,
  clearFragment: () => history.replaceState(null, '', location.pathname),
  goHome: () => location.replace('/'),
});
