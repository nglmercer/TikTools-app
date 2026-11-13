<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, useId, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useControllerCloud, type CloudConnectionModel } from '../model/use-controller-cloud.ts';
const props = defineProps<{ model: CloudConnectionModel }>();
const { t } = useI18n(); const id = useId(); const mode = ref<'teams' | 'join'>('join');
const cloud = useControllerCloud(useControlClient(), props.model);
const { account, signedIn, signingIn, setupAllowed, pending, busy, teams, selected, profile, label, token, error, browserUrl } = cloud;
function moveTab(event: KeyboardEvent): void {
  if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
  event.preventDefault();
  mode.value = event.key === 'Home' ? 'teams' : event.key === 'End' ? 'join' : mode.value === 'teams' ? 'join' : 'teams';
  void nextTick(() => document.getElementById(`${id}-${mode.value}-tab`)?.focus());
}
watch(() => teams.value.length, (length) => { if (length && !pending.value) mode.value = 'teams'; });
onMounted(() => { void cloud.start(); }); onUnmounted(cloud.stop);
</script>

<template>
  <section class="controller__panel controller-cloud">
    <header class="controller-cloud__header">
      <div>
        <h2 class="controller__section-title">
          {{ t('controller.cloud.title') }}
        </h2>
        <p
          v-if="signedIn"
          class="controller__text"
        >
          {{ account?.displayName || account?.email || account?.accountId }}
        </p>
      </div>
      <button
        v-if="signedIn"
        class="controller__button"
        :disabled="busy"
        @click="cloud.logout"
      >
        {{ t('controller.cloud.signOut') }}
      </button>
    </header>
    <template v-if="signingIn">
      <p
        role="status"
        class="controller__text"
      >
        {{ t('controller.cloud.signingIn') }}
      </p>
      <div class="controller__actions">
        <button
          class="controller__button"
          @click="cloud.openLogin"
        >
          {{ t('controller.cloud.openBrowser') }}
        </button>
        <button
          class="controller__button"
          @click="cloud.cancelLogin"
        >
          {{ t('controller.cloud.cancel') }}
        </button>
      </div>
      <details v-if="browserUrl">
        <summary class="controller__summary">
          {{ t('controller.cloud.loginLink') }}
        </summary><code class="controller__code">{{ browserUrl }}</code>
      </details>
    </template>
    <template v-else-if="!signedIn">
      <p class="controller__text">
        {{ t('controller.cloud.signInHelp') }}
      </p>
      <button
        class="controller__button"
        :disabled="busy || !setupAllowed || account?.state === 'unconfigured'"
        @click="cloud.login"
      >
        {{ t('controller.cloud.signIn') }}
      </button>
    </template>
    <template v-else-if="pending">
      <p
        role="status"
        class="controller__text"
      >
        {{ t('controller.cloud.pending') }} <strong>{{ pending.profile }}</strong>
      </p>
      <button
        class="controller__button"
        @click="cloud.cancelPairing"
      >
        {{ t('controller.cloud.cancel') }}
      </button>
    </template>
    <form
      v-else-if="setupAllowed"
      autocomplete="off"
      @submit.prevent="mode === 'join' ? cloud.join() : cloud.connectTeam()"
    >
      <div
        class="controller-cloud__tabs"
        role="tablist"
        :aria-label="t('controller.cloud.title')"
        @keydown="moveTab"
      >
        <button
          :id="`${id}-teams-tab`"
          class="controller__button"
          role="tab"
          type="button"
          :aria-selected="mode === 'teams'"
          :tabindex="mode === 'teams' ? 0 : -1"
          :aria-controls="`${id}-teams`"
          @click="mode = 'teams'"
        >
          {{ t('controller.cloud.teams') }}
        </button>
        <button
          :id="`${id}-join-tab`"
          class="controller__button"
          role="tab"
          type="button"
          :aria-selected="mode === 'join'"
          :tabindex="mode === 'join' ? 0 : -1"
          :aria-controls="`${id}-join`"
          @click="mode = 'join'"
        >
          {{ t('controller.cloud.join') }}
        </button>
      </div>
      <div
        v-if="mode === 'teams'"
        :id="`${id}-teams`"
        role="tabpanel"
        :aria-labelledby="`${id}-teams-tab`"
      >
        <div class="controller__actions">
          <button
            type="button"
            class="controller__button"
            :disabled="busy"
            @click="cloud.refreshTeams"
          >
            {{ t('controller.cloud.refresh') }}
          </button>
        </div>
        <p
          v-if="!teams.length"
          class="controller__text"
        >
          {{ t('controller.cloud.noTeams') }}
        </p>
        <div class="controller-cloud__teams">
          <label
            v-for="team in teams"
            :key="team.id"
            class="controller-cloud__team"
          >
            <input
              v-model="selected"
              type="radio"
              :name="`${id}-team`"
              :value="team.id"
              :disabled="busy"
            >
            <span><strong>{{ team.name }}</strong><small>{{ t(`controller.cloud.roles.${team.role}`) }}</small></span>
          </label>
        </div>
      </div>
      <div
        v-else
        :id="`${id}-join`"
        role="tabpanel"
        :aria-labelledby="`${id}-join-tab`"
      >
        <label class="controller__label">{{ t('controller.cloud.device') }}<input
          v-model="label"
          class="controller__input"
          maxlength="120"
          required
          :disabled="busy"
        ></label>
        <label class="controller__label">{{ t('controller.cloud.invitation') }}<input
          v-model="token"
          class="controller__input"
          type="password"
          autocomplete="new-password"
          minlength="32"
          maxlength="128"
          required
          :disabled="busy"
        ></label>
      </div>
      <label class="controller__label">{{ t('controller.cloud.profile') }}<input
        v-model="profile"
        class="controller__input"
        pattern="[A-Za-z0-9_][A-Za-z0-9_-]{0,63}"
        maxlength="64"
        required
        :disabled="busy"
      ></label>
      <button
        class="controller__button"
        type="submit"
        :disabled="busy || (mode === 'teams' && !selected)"
      >
        {{ mode === 'join' ? t('controller.cloud.request') : t('controller.cloud.connect') }}
      </button>
    </form>
    <p
      v-if="error"
      role="alert"
      class="controller__text controller__error"
    >
      {{ error }}
    </p>
  </section>
</template>

<style scoped>
.controller-cloud__header, .controller-cloud__tabs { display:flex; align-items:center; justify-content:space-between; flex-wrap:wrap; gap:10px; }
.controller-cloud__header > div { min-width:0; flex:1; }
.controller-cloud__header h2 { margin:0; }.controller-cloud__tabs { justify-content:flex-start; margin:12px 0; }
.controller-cloud__tabs [aria-selected=true] { background:var(--color-bg-selected); border-color:var(--color-border-selected); }
.controller-cloud__teams { display:grid; grid-template-columns:repeat(auto-fit,minmax(min(100%,240px),1fr)); gap:10px; }
.controller-cloud__team { display:flex; align-items:center; gap:12px; padding:14px; border:1px solid var(--color-border); border-radius:6px; cursor:pointer; min-width:0; }
.controller-cloud__team:has(input:checked) { border-color:var(--color-border-selected); background:var(--color-bg-selected); }
.controller-cloud__team span { display:grid; gap:4px; overflow-wrap:anywhere; min-width:0; }.controller-cloud__team small { opacity:.8; }
</style>
