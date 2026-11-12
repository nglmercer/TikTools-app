<script setup lang="ts">
import type { ControllerModel } from '../model/controller.ts';
import { useControllerLabels } from '../lib/use-controller-labels.ts';
import type { RemoteProfile } from '../model/controller.ts';
import type { ControllerPairing } from '../model/use-controller-pairing.ts';
const props = defineProps<{ browserMode?: boolean; model: Pick<ControllerModel, 'status' | 'profiles' | 'busy' | 'connect' | 'prepareForget'>; connection: ControllerPairing }>();
const { status, profiles, busy } = props.model;
const { form, fingerprint, pending, pairing, probe, pair } = props.connection;
const { labels, forgetLabels } = useControllerLabels();
const emit = defineEmits<{ forget: [profile: RemoteProfile] }>();
</script>

<template>
  <section
    v-if="!props.browserMode"
    class="controller__panel"
  >
    <h2 class="controller__section-title">
      {{ labels.profiles }}
    </h2><p
      v-if="!profiles.length"
      class="controller__text"
    >
      {{ labels.noProfiles }}
    </p>
    <div
      v-for="profile in profiles"
      :key="profile.name"
      class="controller__profile"
    >
      <div>
        <strong>{{ profile.name }}</strong><p class="controller__text">
          {{ profile.origin }}
        </p>
      </div><button
        class="controller__button"
        :disabled="busy || pairing"
        @click="model.connect(profile.name)"
      >
        {{ labels.connect }}
      </button>
      <button
        class="controller__button"
        :disabled="!model.prepareForget(profile) || pairing"
        @click="emit('forget', profile)"
      >
        {{ forgetLabels.action }}
      </button>
    </div>
  </section>
  <button
    v-if="props.browserMode && ['idle', 'failed'].includes(status.phase)"
    class="controller__button"
    :disabled="busy"
    @click="model.connect('browser')"
  >
    {{ labels.connect }}
  </button>
  <details
    v-if="!props.browserMode && ['idle', 'failed'].includes(status.phase)"
    class="controller__panel"
  >
    <summary class="controller__summary">
      {{ labels.pair }}
    </summary>
    <form
      autocomplete="off"
      @submit.prevent="pair"
    >
      <label class="controller__label">{{ labels.origin }}<input
        v-model="form.origin"
        class="controller__input"
        type="url"
        placeholder="https://host.example"
        required
        :disabled="pairing"
      ></label>
      <label class="controller__label">{{ labels.profile }}<input
        v-model="form.profile"
        class="controller__input"
        pattern="[A-Za-z0-9_][A-Za-z0-9_-]{0,63}"
        maxlength="64"
        required
        :disabled="pairing"
      ></label>
      <label class="controller__label">{{ labels.label }}<input
        v-model="form.label"
        class="controller__input"
        maxlength="128"
        required
        :disabled="pairing"
      ></label>
      <label class="controller__label">{{ labels.invite }}<input
        v-model="form.invitation"
        class="controller__input"
        type="password"
        autocomplete="new-password"
        maxlength="8192"
        required
        :disabled="pairing"
      ></label>
      <button
        class="controller__button"
        type="button"
        :disabled="pairing || !form.origin"
        @click="probe"
      >
        {{ labels.probe }}
      </button>
      <p
        v-if="fingerprint"
        class="controller__text"
      >
        {{ labels.unverified }}<code class="controller__code">{{ fingerprint }}</code>
      </p>
      <label class="controller__label">{{ labels.fingerprint }}<input
        v-model="form.fingerprint"
        class="controller__input"
        maxlength="100"
        :disabled="pairing"
      ></label>
      <label
        v-if="form.fingerprint"
        class="controller__checkbox controller__label"
      ><input
        v-model="form.confirmPin"
        class="controller__input"
        type="checkbox"
        :disabled="pairing"
      >{{ labels.verify }}</label>
      <button
        class="controller__button"
        type="submit"
        :disabled="pairing || !!form.fingerprint && !form.confirmPin"
      >
        {{ labels.submit }}
      </button>
    </form>
  </details>
  <p
    v-if="pending"
    class="controller__text"
    role="status"
  >
    {{ labels.pending }} <code class="controller__code">{{ pending }}</code>
  </p>
</template>
