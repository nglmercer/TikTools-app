/**
 * The browser-side form control boundary used by the Vue frontend.
 *
 * Controls intentionally remain real HTML inputs/selects/textareas. That
 * keeps native caret, selection, keyboard, accessibility, and WebView
 * behavior intact. The bridge gives every control one predictable event
 * contract for code that needs to observe or read forms.
 */

export const TIKTOOLS_CONTROL_EVENT = 'tiktools:control';

export type ControlKind =
  | 'text'
  | 'search'
  | 'password'
  | 'number'
  | 'checkbox'
  | 'radio'
  | 'range'
  | 'select'
  | 'textarea'
  | 'date'
  | 'time'
  | 'color'
  | 'file';

export type ControlValue = string | number | boolean;

export type ControlEventDetail = {
  kind: ControlKind;
  name?: string;
  rawValue: string;
  value: ControlValue;
  source: 'native' | 'programmatic';
};

export type NativeControl = HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement;

export type FormFieldType = 'string' | 'number' | 'integer' | 'boolean' | 'json';

export type FormFieldSchema = FormFieldType | {
  type: FormFieldType;
  defaultValue?: unknown;
};

export type FormSchema = Readonly<Record<string, FormFieldSchema>>;

const installedBridges = new WeakSet<EventTarget>();

export function isNativeControl(value: EventTarget | null): value is NativeControl {
  if (!value || typeof (value as Element).tagName !== 'string') return false;
  const tagName = (value as Element).tagName.toLowerCase();
  return tagName === 'input' || tagName === 'select' || tagName === 'textarea';
}

export function controlKind(control: NativeControl): ControlKind {
  const tagName = control.tagName.toLowerCase();
  if (tagName === 'textarea') return 'textarea';
  if (tagName === 'select') return 'select';
  const type = (control as HTMLInputElement).type.toLowerCase();
  if (type === 'checkbox') return 'checkbox';
  if (type === 'radio') return 'radio';
  if (type === 'number') return 'number';
  if (type === 'range') return 'range';
  if (type === 'search') return 'search';
  if (type === 'password') return 'password';
  if (type === 'date') return 'date';
  if (type === 'time') return 'time';
  if (type === 'color') return 'color';
  if (type === 'file') return 'file';
  return 'text';
}

/**
 * Converts a value before it reaches a native text control. Vue JSX refs are
 * deliberately unwrapped here as a second line of defense, so a ref can
 * never leak into the DOM as "[object Object]".
 */
export function normalizeControlString(value: unknown): string {
  let current = value;
  const seen = new Set<unknown>();
  while (
    current !== null
    && typeof current === 'object'
    && (current as { __v_isRef?: unknown }).__v_isRef === true
    && !seen.has(current)
  ) {
    seen.add(current);
    current = (current as { value?: unknown }).value;
  }

  if (current === null || current === undefined) return '';
  if (typeof current === 'string') return current;
  if (typeof current === 'number' || typeof current === 'boolean' || typeof current === 'bigint') return String(current);
  try {
    const json = JSON.stringify(current);
    return json === undefined ? '' : json;
  } catch {
    return '';
  }
}

export function readNativeControlValue(control: NativeControl): ControlValue {
  const kind = controlKind(control);
  if (kind === 'checkbox' || kind === 'radio') return (control as HTMLInputElement).checked;
  const rawValue = control.value;
  if (kind === 'number' || kind === 'range') {
    if (rawValue.trim() === '') return '';
    const numberValue = Number(rawValue);
    return Number.isFinite(numberValue) ? numberValue : rawValue;
  }
  return rawValue;
}

export function dispatchControlEvent(
  control: NativeControl,
  source: ControlEventDetail['source'] = 'programmatic',
): CustomEvent<ControlEventDetail> {
  const detail: ControlEventDetail = {
    kind: controlKind(control),
    name: control.getAttribute('name') || undefined,
    rawValue: control.value,
    value: readNativeControlValue(control),
    source,
  };
  const event = new CustomEvent<ControlEventDetail>(TIKTOOLS_CONTROL_EVENT, {
    bubbles: true,
    composed: true,
    detail,
  });
  control.dispatchEvent(event);
  return event;
}

function eventControl(event: Event): NativeControl | null {
  const path = typeof event.composedPath === 'function' ? event.composedPath() : [];
  for (const entry of path) {
    if (isNativeControl(entry as EventTarget | null)) return entry as NativeControl;
  }
  return isNativeControl(event.target) ? event.target : null;
}

/**
 * Installs one capturing bridge for every native control in the document.
 * Capturing means raw controls and custom Vue controls follow the same event
 * contract without every consumer having to duplicate an event adapter.
 */
export function installControlEventBridge(root: Document | HTMLElement = document): () => void {
  if (installedBridges.has(root)) return () => undefined;
  installedBridges.add(root);

  const forward = (event: Event): void => {
    const control = eventControl(event);
    if (control) dispatchControlEvent(control, 'native');
  };
  root.addEventListener('input', forward, true);
  root.addEventListener('change', forward, true);

  return () => {
    root.removeEventListener('input', forward, true);
    root.removeEventListener('change', forward, true);
    installedBridges.delete(root);
  };
}

function isTextLikeControl(control: NativeControl): control is HTMLInputElement | HTMLTextAreaElement {
  const tagName = control.tagName.toLowerCase();
  return tagName === 'input' || tagName === 'textarea';
}

/**
 * Synchronizes a controlled value without jumping the user's caret to the
 * end. Controlled Vue renders normally provide the same value after input,
 * but this also handles external updates and async form hydration.
 */
export function syncNativeControlValue(control: NativeControl, value: unknown): void {
  if (controlKind(control) === 'checkbox' || controlKind(control) === 'radio') {
    const checked = value === true || value === 'true';
    const input = control as HTMLInputElement;
    if (input.checked !== checked) input.checked = checked;
    return;
  }

  const next = normalizeControlString(value);
  if (control.value === next) return;

  const focused = typeof document !== 'undefined' && document.activeElement === control;
  const start = isTextLikeControl(control) ? control.selectionStart : null;
  const end = isTextLikeControl(control) ? control.selectionEnd : null;
  control.value = next;

  if (focused && isTextLikeControl(control) && start !== null && end !== null) {
    const nextStart = Math.min(next.length, start);
    const nextEnd = Math.min(next.length, end);
    try {
      control.setSelectionRange(nextStart, nextEnd);
    } catch {
      // Some WebViews reject selection updates for a changing input type.
    }
  }
}

export type SelectSignatureOption = {
  value: string;
  disabled?: boolean;
};

/**
 * Stable semantic signature for select option lists. Only option values and
 * disabled state matter for DOM synchronization: parents may rebuild
 * equivalent arrays with `.map()` on every render, so array identity must
 * never trigger a value reset.
 */
export function selectOptionSignature<T extends SelectSignatureOption>(options: readonly T[]): string {
  return options
    .map((option) => `${option.value}\u0001${option.disabled ? '1' : '0'}`)
    .join('\u0000');
}
/**
 * Focus-boundary helper: moving focus between controls inside a container
 * (password input → Show button, input → select) must not count as leaving
 * the form. Returns true when focus stayed inside `container` (via
 * `relatedTarget`, or — when the WebView reports `null` — via the currently
 * focused element).
 */
export function focusStayedInside(
  container: { contains(node: unknown): boolean },
  relatedTarget: EventTarget | null | undefined,
  activeElement?: unknown,
): boolean {
  for (const candidate of [relatedTarget, activeElement]) {
    if (candidate === null || candidate === undefined) continue;
    try {
      if (container.contains(candidate as Node)) return true;
    } catch {
      // Non-node candidates never count as staying inside.
    }
  }
  return false;
}

function schemaDefinition(field: FormFieldSchema): { type: FormFieldType; defaultValue?: unknown } {
  return typeof field === 'string' ? { type: field } : field;
}

function namedControls(root: ParentNode): Map<string, NativeControl> {
  const controls = new Map<string, NativeControl>();
  const elements = Array.from(root.querySelectorAll<NativeControl>('[name]'));
  if (isNativeControl(root as EventTarget | null)) elements.unshift(root as NativeControl);
  for (const element of elements) {
    const name = element.getAttribute('name');
    if (!name) continue;
    const current = controls.get(name);
    if (!current) {
      controls.set(name, element);
      continue;
    }
    // For radio groups, the checked member is the value-bearing control.
    if ((element as HTMLInputElement).type === 'radio' && (element as HTMLInputElement).checked) controls.set(name, element);
  }
  return controls;
}

/**
 * Reads a named form using a small explicit schema. This keeps forms
 * consistent across native, Vue, process, and future plugin runtimes.
 */
export function readFormValues(root: ParentNode, schema: FormSchema): Record<string, unknown> {
  const controls = namedControls(root);
  const values: Record<string, unknown> = {};
  for (const [name, field] of Object.entries(schema)) {
    const definition = schemaDefinition(field);
    const control = controls.get(name);
    if (!control) {
      values[name] = definition.defaultValue;
      continue;
    }

    const raw = readNativeControlValue(control);
    if (definition.type === 'boolean') {
      values[name] = typeof raw === 'boolean' ? raw : raw === 'true';
    } else if (definition.type === 'number' || definition.type === 'integer') {
      if (raw === '' || (typeof raw === 'string' && raw.trim() === '')) values[name] = definition.defaultValue;
      else {
        const parsed = typeof raw === 'number' ? raw : Number(raw);
        values[name] = Number.isFinite(parsed) ? (definition.type === 'integer' ? Math.trunc(parsed) : parsed) : definition.defaultValue;
      }
    } else if (definition.type === 'json') {
      if (typeof raw !== 'string' || raw.trim() === '') values[name] = definition.defaultValue;
      else {
        try { values[name] = JSON.parse(raw) as unknown; } catch { values[name] = raw; }
      }
    } else {
      values[name] = String(raw);
    }
  }
  return values;
}
