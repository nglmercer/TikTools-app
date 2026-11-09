<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { ControlCallError, errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';

interface CloudVersion {
  version: string;
  target: string;
  size: string;
  yanked: boolean;
  keyRevoked: boolean;
  publisherApproved: boolean;
  advisory: { severity: string; message: string | null; url: string | null } | null;
}
interface CloudPlugin {
  id: string;
  latestStableVersion: string | null;
  metadataPath?: string;
  versions?: CloudVersion[];
}
interface CloudPage {
  generation: string | null;
  items: CloudPlugin[];
  nextCursor: string | null;
}
interface VersionPage {
  generation: string;
  id: string;
  metadataPath: string;
  items: CloudVersion[];
  nextCursor: string | null;
}

type Mode = 'checking' | 'bounded' | 'legacy' | 'unsupported' | 'error';
const { t } = useI18n();
const client = useControlClient();
const mode = ref<Mode>('checking');
const query = ref('');
const activeQuery = ref('');
const page = ref<CloudPage | null>(null);
const cursors = ref<(string | null)[]>([null]);
const pageIndex = ref(0);
const loading = ref(false);
const error = ref('');
const expandedId = ref<string | null>(null);
const versionPage = ref<VersionPage | null>(null);
const versionCursors = ref<(string | null)[]>([null]);
const versionIndex = ref(0);
const versionLoading = ref(false);
const versionError = ref('');
let requestId = 0;
let capabilityId = 0;
let versionRequestId = 0;

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
function positiveDecimal(value: unknown): value is string {
  if (typeof value !== 'string' || !/^[1-9]\d{0,18}$/.test(value)) return false;
  return value.length < 19 || value <= '9223372036854775807';
}
function validCursor(value: unknown): value is string | null {
  return value === null || (typeof value === 'string' && value.length > 0 && value.length <= 2048);
}
function validVersion(version: unknown): version is CloudVersion {
  if (!record(version) || typeof version.version !== 'string' || !version.version || version.version.length > 128 ||
      typeof version.target !== 'string' || !version.target || version.target.length > 128 ||
      !positiveDecimal(version.size) || typeof version.yanked !== 'boolean' ||
      typeof version.keyRevoked !== 'boolean' || typeof version.publisherApproved !== 'boolean') return false;
  return version.advisory === null || (record(version.advisory) &&
    typeof version.advisory.severity === 'string' &&
    ['low', 'moderate', 'high', 'critical'].includes(version.advisory.severity) &&
    typeof version.advisory.message === 'string' && version.advisory.message.length > 0 &&
    (version.advisory.url === null || typeof version.advisory.url === 'string'));
}
function validPage(value: unknown, bounded: boolean): value is CloudPage {
  if (!record(value) || !(value.generation === null || positiveDecimal(value.generation)) ||
      !validCursor(value.nextCursor) || !Array.isArray(value.items) || value.items.length > 50 ||
      (value.generation === null && (value.nextCursor !== null || value.items.length !== 0))) return false;
  return value.items.every((item: unknown) => {
    if (!record(item) || typeof item.id !== 'string' || item.id.length < 2 || item.id.length > 128 ||
        !(item.latestStableVersion === null || (typeof item.latestStableVersion === 'string' && item.latestStableVersion.length > 0 && item.latestStableVersion.length <= 128))) return false;
    if (bounded) return typeof item.metadataPath === 'string' && item.metadataPath.length > 0;
    return Array.isArray(item.versions) && item.versions.length > 0 && item.versions.every(validVersion);
  });
}
function validVersionPage(value: unknown): value is VersionPage {
  return record(value) && positiveDecimal(value.generation) && typeof value.id === 'string' &&
    typeof value.metadataPath === 'string' && validCursor(value.nextCursor) &&
    Array.isArray(value.items) && value.items.length <= 50 && value.items.every(validVersion);
}
function validCapabilities(value: unknown): boolean {
  return record(value) && value.schemaVersion === 1 &&
    value.summaryPath === '/api/v2/catalog/plugin-summaries' &&
    value.versionPagePath === '/api/v2/catalog/plugins/{id}/version-pages' &&
    value.maxItems === 50 && value.maxBytes === 524288 && value.versionOrder === 'ascii-version-target';
}
function clearVersions(): void {
  versionRequestId += 1;
  expandedId.value = null;
  versionPage.value = null;
  versionCursors.value = [null];
  versionIndex.value = 0;
  versionLoading.value = false;
  versionError.value = '';
}
async function initialize(): Promise<void> {
  const current = ++capabilityId;
  mode.value = 'checking';
  error.value = '';
  try {
    const capabilities = await client.call<unknown>('registry.cloudCatalogPaginationCapabilities', {});
    if (current !== capabilityId) return;
    if (!validCapabilities(capabilities)) throw new Error(t('plugins.cloud.invalidResponse'));
    mode.value = 'bounded';
    await load('', null, null, 0);
  } catch (failure) {
    if (current !== capabilityId) return;
    if (failure instanceof ControlCallError && failure.code === 'capability_unavailable' &&
        record(failure.data) && failure.data.reason === 'bounded_catalog_unsupported') {
      mode.value = 'unsupported';
    } else {
      mode.value = 'error';
      error.value = errorMessage(failure);
    }
  }
}
function useLegacy(): void {
  if (mode.value !== 'unsupported') return;
  mode.value = 'legacy';
  void load('', null, null, 0);
}
async function load(search: string, cursor: string | null, generation: string | null, index: number): Promise<void> {
  const request = ++requestId;
  const bounded = mode.value === 'bounded';
  clearVersions();
  loading.value = true;
  error.value = '';
  try {
    const result = await client.call<unknown>(bounded ? 'registry.cloudCatalogSummaries' : 'registry.cloudCatalog', {
      query: search,
      ...(cursor !== null ? { [bounded ? 'cursor' : 'after']: cursor } : {}),
      ...(generation !== null ? { generation } : {}),
    });
    if (request !== requestId) return;
    if (!validPage(result, bounded) || (generation !== null && result.generation !== generation)) {
      throw new Error(t('plugins.cloud.invalidResponse'));
    }
    // Details opened while the next summary was in flight belong to the
    // previous page and must not update the newly displayed generation.
    clearVersions();
    page.value = result;
    activeQuery.value = search;
    if (index === 0) cursors.value = [null];
    else cursors.value = [...cursors.value.slice(0, index), cursor];
    pageIndex.value = index;
  } catch (failure) {
    if (request === requestId) error.value = errorMessage(failure);
  } finally {
    if (request === requestId) loading.value = false;
  }
}
async function loadVersions(item: CloudPlugin, cursor: string | null, index: number): Promise<void> {
  const generation = page.value?.generation;
  if (mode.value !== 'bounded' || !generation || !item.metadataPath) return;
  const request = ++versionRequestId;
  expandedId.value = item.id;
  versionLoading.value = true;
  versionError.value = '';
  try {
    const result = await client.call<unknown>('registry.cloudCatalogVersions', {
      id: item.id, generation, ...(cursor !== null ? { cursor } : {}),
    });
    if (request !== versionRequestId) return;
    if (!validVersionPage(result) || page.value?.generation !== generation ||
        !page.value.items.some((current) => current.id === item.id && current.metadataPath === item.metadataPath) ||
        result.id !== item.id || result.generation !== generation ||
        result.metadataPath !== item.metadataPath) throw new Error(t('plugins.cloud.invalidResponse'));
    versionPage.value = result;
    if (index === 0) versionCursors.value = [null];
    else versionCursors.value = [...versionCursors.value.slice(0, index), cursor];
    versionIndex.value = index;
  } catch (failure) {
    if (request === versionRequestId) versionError.value = errorMessage(failure);
  } finally {
    if (request === versionRequestId) versionLoading.value = false;
  }
}
function toggleVersions(item: CloudPlugin): void {
  if (expandedId.value === item.id) { clearVersions(); return; }
  clearVersions();
  void loadVersions(item, null, 0);
}
function previousVersions(item: CloudPlugin): void {
  if (versionIndex.value === 0 || versionLoading.value) return;
  const index = versionIndex.value - 1;
  void loadVersions(item, versionCursors.value[index] ?? null, index);
}
function nextVersions(item: CloudPlugin): void {
  if (!versionPage.value?.nextCursor || versionLoading.value) return;
  void loadVersions(item, versionPage.value.nextCursor, versionIndex.value + 1);
}
function search(): void {
  if (mode.value === 'bounded' || mode.value === 'legacy') void load(query.value.trim(), null, null, 0);
}
function next(): void {
  const current = page.value;
  if (!current?.nextCursor || !current.generation || loading.value) return;
  void load(activeQuery.value, current.nextCursor, current.generation, pageIndex.value + 1);
}
function previous(): void {
  const current = page.value;
  if (!current?.generation || pageIndex.value === 0 || loading.value) return;
  const index = pageIndex.value - 1;
  void load(activeQuery.value, cursors.value[index] ?? null, current.generation, index);
}
onMounted(() => void initialize());
onUnmounted(() => { capabilityId += 1; requestId += 1; versionRequestId += 1; });
</script>

<template>
  <section
    class="cloud-catalog"
    :aria-label="t('plugins.cloud.title')"
  >
    <h2>{{ t('plugins.cloud.title') }}</h2>
    <p class="cloud-catalog__hint">
      {{ t('plugins.cloud.trustHint') }}
    </p>
    <p
      v-if="mode === 'checking'"
      role="status"
    >
      {{ t('plugins.catalog.searching') }}
    </p>
    <div
      v-if="mode === 'unsupported'"
      class="cloud-catalog__compatibility"
    >
      <p>{{ t('plugins.cloud.legacyHint') }}</p>
      <BaseButton @click="useLegacy">
        {{ t('plugins.cloud.useLegacy') }}
      </BaseButton>
    </div>
    <BaseButton
      v-if="mode === 'error'"
      @click="initialize"
    >
      {{ t('plugins.cloud.retry') }}
    </BaseButton>
    <form
      v-if="mode === 'bounded' || mode === 'legacy'"
      class="cloud-catalog__search"
      @submit.prevent="search"
    >
      <BaseTextField
        v-model="query"
        type="search"
        :aria-label="t('plugins.cloud.search')"
        :placeholder="t('plugins.cloud.search')"
      />
      <BaseButton type="submit">
        {{ t('plugins.catalog.searchButton') }}
      </BaseButton>
    </form>
    <p
      v-if="loading"
      role="status"
    >
      {{ t('plugins.catalog.searching') }}
    </p>
    <p
      v-if="error"
      class="cloud-catalog__error"
      role="alert"
    >
      {{ error }}
    </p>
    <template v-if="page">
      <p class="cloud-catalog__hint">
        {{ t('plugins.cloud.showingFor', { query: activeQuery || t('plugins.cloud.all') }) }}
      </p>
      <p v-if="page.items.length === 0 && !loading">
        {{ t('plugins.cloud.empty') }}
      </p>
      <ul class="cloud-catalog__list">
        <li
          v-for="item in page.items"
          :key="item.id"
          class="cloud-catalog__plugin"
        >
          <h3>{{ item.id }}</h3>
          <p>{{ item.latestStableVersion ? t('plugins.cloud.latest', { version: item.latestStableVersion }) : t('plugins.cloud.noStable') }}</p>
          <BaseButton
            v-if="mode === 'bounded'"
            :aria-expanded="expandedId === item.id"
            @click="toggleVersions(item)"
          >
            {{ expandedId === item.id ? t('plugins.cloud.hideVersions') : t('plugins.cloud.showVersions') }}
          </BaseButton>
          <p
            v-if="expandedId === item.id && versionLoading"
            role="status"
          >
            {{ t('plugins.catalog.searching') }}
          </p>
          <p
            v-if="expandedId === item.id && versionError"
            class="cloud-catalog__error"
            role="alert"
          >
            {{ versionError }}
          </p>
          <ul
            v-if="mode === 'legacy' || expandedId === item.id"
            class="cloud-catalog__versions"
          >
            <li
              v-for="version in mode === 'legacy' ? (item.versions ?? []) : (versionPage?.id === item.id ? versionPage.items : [])"
              :key="`${version.version}/${version.target}`"
            >
              <span>{{ version.version }} · {{ version.target }}</span>
              <span
                v-if="version.yanked"
                class="cloud-catalog__warning"
              >{{ t('plugins.cloud.yanked') }}</span>
              <span
                v-if="version.keyRevoked"
                class="cloud-catalog__warning"
              >{{ t('plugins.cloud.keyRevoked') }}</span>
              <span
                v-if="!version.publisherApproved"
                class="cloud-catalog__warning"
              >{{ t('plugins.cloud.publisherUnapproved') }}</span>
              <span
                v-if="version.advisory"
                class="cloud-catalog__warning"
              >{{ version.advisory.severity }}: {{ version.advisory.message }}</span>
            </li>
          </ul>
          <nav
            v-if="mode === 'bounded' && expandedId === item.id && versionPage?.id === item.id"
            class="cloud-catalog__navigation"
            :aria-label="t('plugins.cloud.versionPages')"
          >
            <BaseButton
              :disabled="versionLoading || versionIndex === 0"
              @click="previousVersions(item)"
            >
              {{ t('plugins.cloud.previous') }}
            </BaseButton>
            <span>{{ t('plugins.cloud.page', { page: versionIndex + 1 }) }}</span>
            <BaseButton
              :disabled="versionLoading || !versionPage.nextCursor"
              @click="nextVersions(item)"
            >
              {{ t('plugins.cloud.next') }}
            </BaseButton>
          </nav>
        </li>
      </ul>
      <nav
        class="cloud-catalog__navigation"
        :aria-label="t('plugins.cloud.pages')"
      >
        <BaseButton
          :disabled="loading || pageIndex === 0"
          @click="previous"
        >
          {{ t('plugins.cloud.previous') }}
        </BaseButton>
        <span>{{ t('plugins.cloud.page', { page: pageIndex + 1 }) }}</span>
        <BaseButton
          :disabled="loading || !page.nextCursor"
          @click="next"
        >
          {{ t('plugins.cloud.next') }}
        </BaseButton>
      </nav>
    </template>
  </section>
</template>

<style scoped>
.cloud-catalog { flex: 1; min-height: 0; overflow: auto; padding: var(--space-4); }
.cloud-catalog h2, .cloud-catalog h3 { margin: 0; overflow-wrap: anywhere; }
.cloud-catalog__hint { color: var(--color-text-muted); }
.cloud-catalog__search { display: flex; gap: var(--space-2); max-width: 36rem; }
.cloud-catalog__search > :first-child { flex: 1; min-width: 0; }
.cloud-catalog__list, .cloud-catalog__versions { list-style: none; padding: 0; }
.cloud-catalog__plugin { border-bottom: var(--border-width) solid var(--color-border-subtle); padding: var(--space-3) 0; }
.cloud-catalog__versions li { display: flex; flex-wrap: wrap; gap: var(--space-2); overflow-wrap: anywhere; }
.cloud-catalog__warning { color: var(--color-warning); }
.cloud-catalog__error { color: var(--color-danger-hover); overflow-wrap: anywhere; }
.cloud-catalog__navigation { display: flex; gap: var(--space-3); align-items: center; }
</style>
