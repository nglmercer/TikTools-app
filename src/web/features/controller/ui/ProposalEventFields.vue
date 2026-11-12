<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { MessageSchema } from '@/shared/i18n/i18n.ts';
import type { ProposalModel } from '../model/proposals.ts';
import type { ProposalBuilderDraft } from '../model/use-proposal-builder-draft.ts';
const props = defineProps<{
  model: Pick<ProposalModel, 'busy'>;
  fields: Pick<ProposalBuilderDraft,
    'trigger' | 'customTopic' | 'contains' | 'giftContains' | 'actionId' | 'runMode' | 'cooldown' | 'cooldownScope'>;
}>();
const { tm } = useI18n();
const labels = computed(() => tm('controller.proposalBuilder.labels') as MessageSchema['controller']['proposalBuilder']['labels']);
const { trigger, customTopic, contains, giftContains, actionId, runMode, cooldown, cooldownScope } = props.fields;
</script>

<template>
  <label>{{ labels.trigger }}<select
    v-model="trigger"
    :disabled="model.busy.value"
  ><option value="tiktok.chat">{{ labels.chat }}</option><option value="tiktok.gift">{{ labels.gift }}</option><option value="tiktok.like">{{ labels.like }}</option><option value="tiktok.follow">{{ labels.follow }}</option><option value="tiktok.share">{{ labels.share }}</option><option value="tiktok.join">{{ labels.join }}</option><option value="custom">{{ labels.custom }}</option></select></label>
  <label v-if="trigger === 'custom'">{{ labels.topic }}<input
    v-model="customTopic"
    maxlength="64"
    spellcheck="false"
    :disabled="model.busy.value"
  ></label>
  <label v-if="trigger === 'tiktok.chat'">{{ labels.contains }}<input
    v-model="contains"
    maxlength="500"
    :disabled="model.busy.value"
  ></label>
  <label v-if="trigger === 'tiktok.gift'">{{ labels.giftContains }}<input
    v-model="giftContains"
    maxlength="500"
    :disabled="model.busy.value"
  ></label>
  <label>{{ labels.actionId }}<input
    v-model="actionId"
    maxlength="128"
    spellcheck="false"
    :disabled="model.busy.value"
  ></label>
  <label>{{ labels.runMode }}<select
    v-model="runMode"
    :disabled="model.busy.value"
  ><option value="all">{{ labels.runAll }}</option><option value="random">{{ labels.runRandom }}</option></select></label>
  <label>{{ labels.cooldown }}<input
    v-model="cooldown"
    type="number"
    min="0"
    max="86400000"
    step="1"
    :disabled="model.busy.value"
  ></label>
  <label>{{ labels.cooldownScope }}<select
    v-model="cooldownScope"
    :disabled="model.busy.value"
  ><option value="global">{{ labels.cooldownGlobal }}</option><option value="user">{{ labels.cooldownViewer }}</option></select></label>
</template>

<style scoped src="./proposal-builder.css"></style>
