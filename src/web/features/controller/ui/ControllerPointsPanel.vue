<script setup lang="ts">
import type { ControllerModel } from '../model/controller.ts';
import { useControllerLabels } from '../lib/use-controller-labels.ts';
import { computed, useTemplateRef } from 'vue';
const props = defineProps<{ model: Pick<ControllerModel, 'status' | 'points' | 'pointsError' | 'pointsAllowed' | 'busy' | 'ready' | 'selectPoints' | 'preparePoints' | 'adjustPoints' | 'pointsCanAdjust'> }>();
const { status, points, pointsError, pointsAllowed, busy, ready } = props.model;
const pointSearch = defineModel<string>('search', { required: true });
const pointAmount = defineModel<string | number>('amount', { required: true });
const pointsSection = useTemplateRef('pointsSection');
const { pts } = useControllerLabels();
const pointId = computed(() => points.value?.selectedId ?? '');
async function requestPoints(): Promise<void> {
  if (!String(pointAmount.value).trim()) return;
  const target = props.model.preparePoints(pointId.value, Number(pointAmount.value));
  if (target && await props.model.adjustPoints(target)) pointAmount.value = '';
}
defineExpose({ focus: () => {
  pointsSection.value?.focus();
  pointsSection.value?.scrollIntoView?.({ block: 'start' });
} });
</script>

<template>
  <section
    v-if="ready && pointsAllowed"
    ref="pointsSection"
    class="controller__panel controller__points"
    :aria-label="pts.title"
    tabindex="-1"
  >
    <h2 class="controller__section-title">
      {{ pts.title }}
    </h2>
    <p
      v-if="pointsError"
      class="controller__text"
      role="alert"
    >
      {{ pointsError }}
    </p>
    <form
      class="controller__pointSearch"
      @submit.prevent="model.selectPoints(pointSearch)"
    >
      <label class="controller__label">{{ pts.search }}<input
        v-model="pointSearch"
        class="controller__input"
        maxlength="128"
        autocomplete="off"
        spellcheck="false"
        :disabled="busy"
      ></label>
      <button
        class="controller__button"
        type="submit"
        :disabled="busy || !pointSearch.trim()"
      >
        {{ pts.find }}
      </button>
      <button
        v-if="points?.selectedId"
        class="controller__button"
        type="button"
        @click="pointSearch='';model.selectPoints('')"
      >
        {{ pts.clear }}
      </button>
    </form>
    <p
      v-if="!points || points.loading"
      class="controller__text"
      role="status"
    >
      {{ pts.loading }}
    </p>
    <template v-if="points && !points.loading && !points.errorCode">
      <h3 v-if="!points.selectedId">
        {{ pts.top }} ({{ points.rows?.length ?? 0 }})
      </h3>
      <p
        v-if="!points.selectedId && !points.rows?.length"
        class="controller__text"
      >
        {{ pts.empty }}
      </p>
      <ul class="controller__pointRows">
        <li
          v-for="item in points.rows"
          :key="item.version.id"
        >
          <button
            class="controller__button"
            :aria-pressed="points.selectedId===item.version.id"
            @click="pointSearch=item.version.id;model.selectPoints(item.version.id)"
          >
            {{ item.version.id }} · {{ item.viewer.points }} {{ points.currencyName }}
          </button>
        </li>
      </ul>
      <div
        v-if="points.selectedId"
        class="controller__pointTarget"
      >
        <h3>{{ pts.selected }}: {{ points.selectedId }}</h3>
        <p
          v-if="points.selected"
          class="controller__text"
        >
          {{ pts.result }}: {{ points.selected.viewer.points }} {{ points.currencyName }}
        </p>
        <p
          v-else-if="points.selectedVersion?.revision==='0'"
          class="controller__text"
        >
          {{ pts.create }}
        </p>
        <form
          v-if="status.hello?.capabilities.includes('points.adjust') && status.hello.features['pointsAdjust']"
          @submit.prevent="requestPoints"
        >
          <label class="controller__label">{{ pts.amount }} ({{ points.currencyName }})<input
            v-model="pointAmount"
            class="controller__input"
            type="number"
            step="0.01"
            min="-1000000000"
            max="1000000000"
            :disabled="!model.pointsCanAdjust()"
          ></label>
          <button
            class="controller__button"
            type="submit"
            :disabled="!model.preparePoints(pointId, Number(pointAmount)) || !String(pointAmount).trim()"
          >
            {{ pts.adjust }}
          </button>
        </form>
      </div>
    </template>
  </section>
</template>
