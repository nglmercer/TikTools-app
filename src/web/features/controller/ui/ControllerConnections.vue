<script setup lang="ts">
import ControllerCloudConnections from './ControllerCloudConnections.vue';
import type { ControllerModel } from '../model/controller.ts';
import { useControllerLabels } from '../lib/use-controller-labels.ts';
import type { RemoteProfile } from '../model/controller.ts';
const props = defineProps<{ browserMode?: boolean; model: Pick<ControllerModel, 'status' | 'profiles' | 'busy' | 'connect' | 'prepareForget' | 'inventory' | 'refresh' | 'disconnect'> }>();
const { status, profiles, busy } = props.model;
const { labels, forgetLabels } = useControllerLabels();
const emit = defineEmits<{ forget: [profile: RemoteProfile] }>();
</script>

<template>
  <ControllerCloudConnections
    v-if="!props.browserMode && status.cloudSupported"
    :model="model"
  />
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
        :disabled="busy"
        @click="model.connect(profile.name)"
      >
        {{ labels.connect }}
      </button>
      <button
        class="controller__button"
        :disabled="!model.prepareForget(profile)"
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
</template>
