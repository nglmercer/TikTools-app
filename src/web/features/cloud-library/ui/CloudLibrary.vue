<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { ControlCallError, errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useCloudContext } from '@/entities/cloud-account/model/cloud-context.ts';
import { useLibraryImport } from '@/entities/cloud-bundles/model/use-library-import.ts';
import { createRequestScope } from '@/shared/lib/request-scope.ts';
import { localizedText } from '@/shared/i18n/i18n.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';

type Scope = 'public' | 'mine';
type Kind = 'template' | 'profile';
type Status = 'draft' | 'pending' | 'published' | 'rejected' | 'withdrawn';
interface Meta { id: string; kind: Kind; slug: string; revision: string; title: string; sha256: string; status: Status }
interface Page { items: Meta[]; nextCursor: string | null }
interface Document { meta: Meta; document: unknown }

const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const decimal = /^[1-9][0-9]*$/;
const digest = /^[0-9a-f]{64}$/;
const kinds = new Set(['template', 'profile']);
const statuses = new Set(['draft', 'pending', 'published', 'rejected', 'withdrawn']);
function record(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}
function meta(value: unknown): value is Meta {
  if (!record(value) || Object.keys(value).sort().join(',') !== 'id,kind,revision,sha256,slug,status,title') return false;
  return typeof value.id === 'string' && uuid.test(value.id)
    && typeof value.kind === 'string' && kinds.has(value.kind)
    && typeof value.slug === 'string' && value.slug.length > 0 && value.slug.length <= 256
    && typeof value.title === 'string' && value.title.length > 0 && value.title.length <= 1024
    && typeof value.revision === 'string' && decimal.test(value.revision) && value.revision.length <= 19
    && (value.revision.length < 19 || value.revision <= '9223372036854775807')
    && typeof value.sha256 === 'string' && digest.test(value.sha256)
    && typeof value.status === 'string' && statuses.has(value.status);
}
function page(value: unknown, scope: Scope): value is Page {
  if (!record(value) || Object.keys(value).sort().join(',') !== 'items,nextCursor'
    || !Array.isArray(value.items) || value.items.length > 50
    || !value.items.every((item) => meta(item) && (scope !== 'public' || item.status === 'published'))
    || !(value.nextCursor === null || (typeof value.nextCursor === 'string' && uuid.test(value.nextCursor)))) return false;
  return new Set(value.items.map((item: Meta) => item.id)).size === value.items.length;
}
function document(value: unknown, expected: Meta): value is Document {
  if (!record(value) || Object.keys(value).sort().join(',') !== 'document,meta' || !meta(value.meta)) return false;
  return value.meta.id === expected.id && value.meta.revision === expected.revision
    && value.meta.sha256 === expected.sha256 && value.meta.kind === expected.kind
    && value.meta.status === expected.status && value.meta.title === expected.title
    && value.meta.slug === expected.slug && record(value.document);
}

withDefaults(defineProps<{
  /** Without the group box and its title, for a host (a dialog tab) that already names the area. */
  bare?: boolean;
}>(), { bare: false });

const { t } = useI18n();
const client = useControlClient();
const account = useAccountStore();
const libraryImport = useLibraryImport();
const scope = ref<Scope>('public');
const kind = ref<Kind | ''>('');
const query = ref('');
const activeQuery = ref('');
const activeKind = ref<Kind | ''>('');
const shown = ref<Page | null>(null);
const selected = ref<Document | null>(null);
const loading = ref(false);
const detailLoading = ref(false);
const error = ref('');
const detailError = ref('');
const importing = ref(false);
const staged = ref(false);
const importError = ref('');
const cursor = ref<string | null>(null);
const history = ref<(string | null)[]>([]);
const mineAvailable = computed(() => account.status?.state === 'signed-in' && !account.working && !account.loading);
const detail = computed(() => {
  const value = selected.value?.document;
  if (!record(value)) return null;
  const event = record(value.event) ? value.event : null;
  const names = Array.isArray(value.actions)
    ? value.actions.map((action: unknown) => record(action) && typeof action.name === 'string' ? action.name : '').filter(Boolean)
    : [];
  const templates = Array.isArray(value.templates)
    ? value.templates.map((entry: unknown) => record(entry) && record(entry.template) ? localizedText(entry.template.title) : '').filter(Boolean)
    : [];
  return {
    title: localizedText(value.title) || (typeof value.name === 'string' ? value.name : ''),
    description: localizedText(value.description),
    event: event && typeof event.name === 'string' ? event.name : '',
    names, templates,
  };
});
// Lists, one open document and the import each have their own generation: closing a
// detail never discards the list, and none of them can answer for a newer view.
const listScope = createRequestScope();
const detailScope = createRequestScope();
const importScope = createRequestScope();
function clear(): void {
  listScope.invalidate(); detailScope.invalidate(); importScope.invalidate();
  shown.value = null; selected.value = null; cursor.value = null; history.value = [];
  loading.value = false; detailLoading.value = false; error.value = ''; detailError.value = '';
  importing.value = false; staged.value = false; importError.value = '';
}
async function load(next: string | null, previous: (string | null)[]): Promise<void> {
  const requestedScope = scope.value;
  if (requestedScope === 'mine' && !mineAvailable.value) return;
  const ticket = listScope.begin();
  detailScope.invalidate(); importScope.invalidate();
  const requestedKind = activeKind.value || null;
  const requestedQuery = activeQuery.value;
  loading.value = true; detailLoading.value = false; selected.value = null; error.value = ''; detailError.value = '';
  importing.value = false; staged.value = false; importError.value = '';
  try {
    const result = await client.call<unknown>('cloudLibrary.list', {
      scope: requestedScope, kind: requestedKind, query: requestedQuery || null, after: next,
    });
    if (!ticket.isCurrent()) return;
    if (!page(result, requestedScope)) throw new Error('invalid page');
    shown.value = result; cursor.value = next; history.value = previous;
  } catch {
    if (ticket.isCurrent()) error.value = t('cloudLibrary.unavailable');
  } finally { if (ticket.isCurrent()) loading.value = false; }
}
function search(): void { activeQuery.value = query.value.trim(); activeKind.value = kind.value; clear(); void load(null, []); }
function nextPage(): void { if (shown.value?.nextCursor) void load(shown.value.nextCursor, [...history.value, cursor.value]); }
function previousPage(): void {
  if (history.value.length) void load(history.value[history.value.length - 1]!, history.value.slice(0, -1));
}
async function open(item: Meta): Promise<void> {
  if (loading.value || detailLoading.value || (scope.value === 'mine' && !mineAvailable.value)) return;
  const ticket = detailScope.begin();
  importScope.invalidate();
  const requestedScope = scope.value;
  detailLoading.value = true; selected.value = null; detailError.value = ''; staged.value = false; importError.value = ''; importing.value = false;
  try {
    const result = await client.call<unknown>('cloudLibrary.read', { scope: requestedScope, id: item.id, revision: item.revision });
    if (!ticket.isCurrent()) return;
    if (!document(result, item)) throw new Error('invalid document');
    selected.value = result;
  } catch { if (ticket.isCurrent()) detailError.value = t('cloudLibrary.unavailable'); }
  finally { if (ticket.isCurrent()) detailLoading.value = false; }
}

/** The review is the single owner-shell dialog; this resolves when the owner confirmed or discarded it. */
async function importTemplate(): Promise<void> {
  const current = selected.value;
  if (!current || importing.value || current.meta.kind !== 'template') return;
  const ticket = importScope.begin();
  importing.value = true; importError.value = ''; staged.value = false;
  try {
    const source = { id: current.meta.id, revision: current.meta.revision, scope: scope.value, record: current };
    await libraryImport.importTemplate(source);
    if (ticket.isCurrent()) staged.value = true;
  } catch (failure) {
    if (!ticket.isCurrent()) return;
    const code = failure instanceof ControlCallError ? failure.code : '';
    importError.value = code === 'signed_out' ? t('cloudLibrary.signIn') : code === 'cancelled' ? '' : errorMessage(failure);
  } finally {
    if (ticket.isCurrent()) importing.value = false;
  }
}
watch(scope, () => search());
// A logout and login of the same account, an account action in progress, the host's context
// notice and a gap in the event stream (a missed notice) all end the private content.
// The notice is advisory for the same host, not a cross-process delivery barrier.
useCloudContext(() => { if (scope.value === 'mine') clear(); });
onMounted(search);
onUnmounted(clear);
</script>

<template>
  <component
    :is="bare ? 'div' : BaseGroupBox"
    v-bind="bare ? { class: 'cloud-library' } : { title: t('cloudLibrary.title'), description: t('cloudLibrary.description') }"
  >
    <form
      class="cloud-library__filters"
      @submit.prevent="search"
    >
      <label>{{ t('cloudLibrary.scope') }}
        <select v-model="scope">
          <option value="public">{{ t('cloudLibrary.public') }}</option>
          <option
            value="mine"
            :disabled="!mineAvailable"
          >{{ t('cloudLibrary.mine') }}</option>
        </select>
      </label>
      <label>{{ t('cloudLibrary.kind') }}
        <select v-model="kind"><option value="">{{ t('cloudLibrary.all') }}</option><option value="template">{{ t('cloudLibrary.template') }}</option><option value="profile">{{ t('cloudLibrary.profile') }}</option></select>
      </label>
      <label>{{ t('cloudLibrary.query') }} <input
        v-model="query"
        maxlength="100"
        type="search"
      ></label>
      <button
        type="submit"
        :disabled="loading || (scope === 'mine' && !mineAvailable)"
      >
        {{ t('cloudLibrary.search') }}
      </button>
    </form>
    <p
      v-if="scope === 'mine' && !mineAvailable"
      role="status"
    >
      {{ t('cloudLibrary.signIn') }}
    </p>
    <p
      v-if="loading"
      role="status"
    >
      {{ t('cloudLibrary.loading') }}
    </p>
    <p
      v-if="error"
      role="alert"
    >
      {{ error }}
    </p>
    <template v-if="shown">
      <p v-if="!shown.items.length">
        {{ t('cloudLibrary.empty') }}
      </p>
      <ul
        v-else
        class="cloud-library__list"
      >
        <li
          v-for="item in shown.items"
          :key="item.id"
        >
          <button
            type="button"
            :disabled="loading || detailLoading"
            @click="open(item)"
          >
            {{ item.title }}
          </button>
          <span>{{ t(`cloudLibrary.${item.kind}`) }} · {{ t(`cloudLibrary.status.${item.status}`) }} · {{ t('cloudLibrary.revision', { revision: item.revision }) }}</span>
        </li>
      </ul>
      <nav :aria-label="t('cloudLibrary.pages')">
        <button
          type="button"
          :disabled="loading || !history.length"
          @click="previousPage"
        >
          {{ t('cloudLibrary.previous') }}
        </button>
        <button
          type="button"
          :disabled="loading || !shown.nextCursor"
          @click="nextPage"
        >
          {{ t('cloudLibrary.next') }}
        </button>
      </nav>
    </template>
    <p
      v-if="detailLoading"
      role="status"
    >
      {{ t('cloudLibrary.loading') }}
    </p>
    <p
      v-if="detailError"
      role="alert"
    >
      {{ detailError }}
    </p>
    <section
      v-if="selected"
      class="cloud-library__detail"
      :aria-label="selected.meta.title"
    >
      <h3>{{ selected.meta.title }}</h3>
      <p>{{ t('cloudLibrary.revision', { revision: selected.meta.revision }) }} · {{ t(`cloudLibrary.status.${selected.meta.status}`) }}</p>
      <p v-if="detail?.title && detail.title !== selected.meta.title">
        {{ detail.title }}
      </p>
      <p v-if="detail?.description">
        {{ detail.description }}
      </p>
      <p v-if="detail?.event">
        {{ t('cloudLibrary.event', { name: detail.event }) }}
      </p>
      <div v-if="detail?.names.length">
        <p>{{ t('cloudLibrary.actions') }}</p><ul>
          <li
            v-for="(name, index) in detail.names"
            :key="index"
          >
            {{ name }}
          </li>
        </ul>
      </div>
      <div v-if="detail?.templates.length">
        <p>{{ t('cloudLibrary.templates') }}</p><ul>
          <li
            v-for="(name, index) in detail.templates"
            :key="index"
          >
            {{ name }}
          </li>
        </ul>
      </div>
      <template v-if="selected.meta.kind === 'template'">
        <BaseButton
          :disabled="importing"
          @click="importTemplate"
        >
          {{ t('cloudLibrary.import') }}
        </BaseButton>
        <p
          v-if="staged"
          role="status"
        >
          {{ t('cloudLibrary.staged') }}
        </p>
        <p
          v-if="importError"
          role="alert"
        >
          {{ importError }}
        </p>
      </template>
      <button
        type="button"
        @click="selected = null"
      >
        {{ t('common.close') }}
      </button>
    </section>
  </component>
</template>

<style scoped>
.cloud-library__filters { display: flex; flex-wrap: wrap; gap: var(--space-3); align-items: end; }
.cloud-library__filters label { display: grid; gap: var(--space-1); }
.cloud-library__list { margin: var(--space-3) 0; padding: 0; list-style: none; border: var(--border-width) solid var(--color-border-subtle); border-radius: var(--radius-sm); }
.cloud-library__list li { display: flex; flex-wrap: wrap; gap: var(--space-2) var(--space-4); align-items: center; justify-content: space-between; padding: var(--space-2) var(--space-3); }
.cloud-library__list li + li { border-top: var(--border-width) solid var(--color-border-subtle); }
.cloud-library__list li > button { padding: 0; border: 0; background: transparent; font-weight: var(--font-weight-semibold); text-align: left; }
.cloud-library__list li > button:hover:not(:disabled) { text-decoration: underline; background: transparent; }
.cloud-library__list li > span { color: var(--color-text-muted); font-size: var(--font-size-sm); }
.cloud-library__detail { margin-top: var(--space-3); }
.cloud-library__detail { overflow-wrap: anywhere; }
nav { display: flex; gap: var(--space-2); }
</style>
