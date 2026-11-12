<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { MessageSchema } from '@/shared/i18n/i18n.ts';
import type { ProposalModel } from '../model/proposals.ts';
import type { ProposalBuilderDraft } from '../model/use-proposal-builder-draft.ts';
const props = defineProps<{
  model: Pick<ProposalModel, 'busy'>;
  fields: Pick<ProposalBuilderDraft,
    'actionType' | 'message' | 'milliseconds' | 'emitTopic' | 'emitData' | 'fetchUrl' | 'fetchMethod' | 'fetchHeaders' | 'fetchBody' | 'fetchTimeout' | 'fetchResponse' | 'fetchPrivate' | 'audioPath' | 'audioVolume' | 'audioOverlap' | 'scriptSource' | 'viewerTarget' | 'viewer' | 'delta'>;
}>();
const { tm } = useI18n();
const labels = computed(() => tm('controller.proposalBuilder.labels') as MessageSchema['controller']['proposalBuilder']['labels']);
const { actionType, message, milliseconds, emitTopic, emitData, fetchUrl, fetchMethod, fetchHeaders, fetchBody, fetchTimeout, fetchResponse, fetchPrivate, audioPath, audioVolume, audioOverlap, scriptSource, viewerTarget, viewer, delta } = props.fields;
</script>

<template>
  <label>{{ labels.actionType }}<select
    v-model="actionType"
    :disabled="model.busy.value"
  ><option value="core.log">{{ labels.log }}</option><option value="core.delay">{{ labels.delay }}</option><option value="core.points">{{ labels.points }}</option><option value="core.emit">{{ labels.emit }}</option><option value="core.fetch">{{ labels.fetch }}</option><option value="audio.play">{{ labels.audio }}</option><option value="core.code">{{ labels.script }}</option></select></label>
  <label v-if="actionType === 'core.log'">{{ labels.message }}<input
    v-model="message"
    maxlength="8192"
    :disabled="model.busy.value"
  ></label>
  <label v-else-if="actionType === 'core.delay'">{{ labels.milliseconds }}<input
    v-model="milliseconds"
    type="number"
    min="0"
    max="60000"
    step="1"
    :disabled="model.busy.value"
  ></label>
  <template v-else-if="actionType === 'core.emit'">
    <label>{{ labels.topic }}<input
      v-model="emitTopic"
      maxlength="64"
      spellcheck="false"
      :disabled="model.busy.value"
    ></label>
    <label>{{ labels.emitData }}<textarea
      v-model="emitData"
      rows="4"
      spellcheck="false"
      :disabled="model.busy.value"
    /></label>
  </template>
  <template v-else-if="actionType === 'core.fetch'">
    <label>{{ labels.fetchUrl }}<input
      v-model="fetchUrl"
      maxlength="8192"
      spellcheck="false"
      autocomplete="off"
      :disabled="model.busy.value"
    ></label>
    <label>{{ labels.fetchMethod }}<select
      v-model="fetchMethod"
      :disabled="model.busy.value"
    ><option>GET</option><option>POST</option><option>PUT</option><option>DELETE</option></select></label>
    <label>{{ labels.fetchHeaders }}<textarea
      v-model="fetchHeaders"
      rows="4"
      spellcheck="false"
      autocomplete="off"
      :disabled="model.busy.value"
    /></label>
    <label>{{ labels.fetchBody }}<textarea
      v-model="fetchBody"
      rows="4"
      spellcheck="false"
      autocomplete="off"
      :disabled="model.busy.value"
    /></label>
    <label>{{ labels.fetchTimeout }}<input
      v-model="fetchTimeout"
      type="number"
      min="100"
      max="120000"
      step="1"
      :disabled="model.busy.value"
    ></label>
    <label>{{ labels.fetchResponse }}<input
      v-model="fetchResponse"
      type="number"
      min="1"
      max="2097152"
      step="1"
      :disabled="model.busy.value"
    ></label>
    <label><input
      v-model="fetchPrivate"
      type="checkbox"
      :disabled="model.busy.value"
    >{{ labels.fetchPrivate }}</label>
  </template>
  <template v-else-if="actionType === 'audio.play'">
    <label>{{ labels.audioPath }}<input
      v-model="audioPath"
      maxlength="4096"
      spellcheck="false"
      autocomplete="off"
      :disabled="model.busy.value"
    ></label>
    <label>{{ labels.audioVolume }}<input
      v-model="audioVolume"
      type="number"
      min="0"
      max="1"
      step="any"
      :disabled="model.busy.value"
    ></label>
    <label>{{ labels.audioOverlap }}<select
      v-model="audioOverlap"
      :disabled="model.busy.value"
    ><option value="allow">{{ labels.audioAllow }}</option><option value="restart">{{ labels.audioRestart }}</option><option value="drop">{{ labels.audioDrop }}</option></select></label>
  </template>
  <template v-else-if="actionType === 'core.code'">
    <label>{{ labels.scriptSource }}<textarea
      v-model="scriptSource"
      rows="10"
      spellcheck="false"
      autocomplete="off"
      :disabled="model.busy.value"
    /></label>
  </template>
  <template v-else>
    <label>{{ labels.viewerTarget }}<select
      v-model="viewerTarget"
      :disabled="model.busy.value"
    ><option value="specific">{{ labels.specificViewer }}</option><option value="event">{{ labels.triggeringViewer }}</option></select></label>
    <label v-if="viewerTarget === 'specific'">{{ labels.viewer }}<input
      v-model="viewer"
      maxlength="128"
      spellcheck="false"
      autocomplete="off"
      :disabled="model.busy.value"
    ></label>
    <label>{{ labels.delta }}<input
      v-model="delta"
      type="number"
      min="-1000000000"
      max="1000000000"
      step="any"
      :disabled="model.busy.value"
    ></label>
  </template>
</template>

<style scoped src="./proposal-builder.css"></style>
