import { GUIDE_SCOPE_ATTR, GUIDE_TARGET_ATTR } from '../../../../../../src/web/features/guided-help/model/guide-dom.ts';

/** Marks mounted fixtures so every suite can remove them in afterEach. */
const FIXTURE_MARKER = 'data-test-guide';

/** Appends a `section[data-guide-scope]` to the document, optionally filled with markup. */
export function mountGuideScope(scope: string, markup = ''): HTMLElement {
  const root = document.createElement('section');
  root.setAttribute(GUIDE_SCOPE_ATTR, scope);
  root.setAttribute(FIXTURE_MARKER, '');
  root.innerHTML = markup;
  document.body.append(root);
  return root;
}

/** A scope whose children are `data-guide` targets, one per name. */
export function mountGuideTargets(scope: string, targets: readonly string[], tag: 'button' | 'section' = 'button'): HTMLElement {
  const root = mountGuideScope(scope);
  for (const name of targets) {
    const target = document.createElement(tag);
    target.setAttribute(GUIDE_TARGET_ATTR, name);
    root.append(target);
  }
  return root;
}

/** Removes every fixture created by the helpers above. */
export function removeGuideFixtures(): void {
  document.querySelectorAll(`[${FIXTURE_MARKER}]`).forEach((element) => element.remove());
}
