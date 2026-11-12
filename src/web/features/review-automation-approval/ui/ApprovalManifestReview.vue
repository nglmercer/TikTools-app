<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import BaseJsonView from '@/shared/ui/json/BaseJsonView.vue';
import type { CapturedApprovalReview } from '../model/contracts.ts';
defineProps<{ review: Pick<CapturedApprovalReview, 'manifest' | 'subjects'> }>();
const { t } = useI18n();
const stateLabel = (state: boolean | null): string => t(`automationApproval.${state === null ? 'unknown' : state ? 'enabled' : 'disabled'}`);
</script>

<template>
  <section
    class="approval-section"
    data-test="approval-manifest"
  >
    <h3 class="approval-text">
      {{ t('automationApproval.manifest') }}
    </h3>
    <p class="approval-text approval-paragraph">
      {{ t('automationApproval.manifestLead') }}
    </p>
    <template v-if="review.manifest">
      <p class="approval-text approval-paragraph">
        {{ t('automationApproval.policy', { version: review.manifest.policyVersion }) }}
      </p>
      <p class="approval-identity approval-text approval-paragraph">
        {{ t('automationApproval.digest', { digest: review.manifest.manifestDigest }) }}
      </p>
      <h4 class="approval-text approval-heading">
        {{ t('automationApproval.reasons') }}
      </h4>
      <ul class="approval-list">
        <li
          v-for="reason in review.manifest.reasons"
          :key="reason"
          class="approval-list-item"
        >
          {{ t(`automationApproval.reason.${reason}`) }}
        </li>
      </ul>
      <h4 class="approval-text approval-heading">
        {{ t('automationApproval.effects', { count: review.manifest.effects.length }) }}
      </h4>
      <article
        v-for="effect in review.manifest.effects"
        :key="effect.id"
        class="approval-card"
        data-test="approval-effect"
      >
        <h4 class="approval-text approval-heading">
          {{ effect.id }} · {{ effect.typeId ?? effect.kind }}
        </h4>
        <p class="approval-text approval-paragraph">
          {{ t(`automationApproval.${effect.decision}`) }} · {{ stateLabel(effect.enabled) }}
        </p>
        <p
          v-if="effect.dynamicTarget"
          class="approval-text approval-paragraph"
        >
          {{ t('automationApproval.dynamic') }}
        </p>
        <h5 class="approval-heading">
          {{ t('automationApproval.fields') }}
        </h5>
        <ul
          v-if="effect.renderFields.length"
          class="approval-list"
        >
          <li
            v-for="field in effect.renderFields"
            :key="field"
            class="approval-list-item"
          >
            <code>{{ field }}</code>
          </li>
        </ul>
        <p
          v-else
          class="approval-text approval-paragraph"
        >
          {{ t('automationApproval.noFields') }}
        </p>
        <h5 class="approval-heading">
          {{ t('automationApproval.capabilities') }}
        </h5>
        <ul
          v-if="effect.requiredCapabilities.length"
          class="approval-list"
        >
          <li
            v-for="capability in effect.requiredCapabilities"
            :key="capability"
            class="approval-list-item"
          >
            <code>{{ capability }}</code>
          </li>
        </ul>
        <p
          v-else
          class="approval-text approval-paragraph"
        >
          {{ t('automationApproval.noCapabilities') }}
        </p>
        <ul
          v-if="effect.reasons.length"
          class="approval-list"
        >
          <li
            v-for="reason in effect.reasons"
            :key="reason"
            class="approval-list-item"
          >
            {{ t(`automationApproval.reason.${reason}`) }}
          </li>
        </ul>
      </article>
    </template>
    <p
      v-else
      class="approval-text approval-paragraph"
    >
      {{ t('automationApproval.unavailableManifest') }}
    </p>
  </section>

  <section
    class="approval-section"
    data-test="approval-subjects"
  >
    <h3 class="approval-text">
      {{ t('automationApproval.subjects', { count: review.subjects.length }) }}
    </h3>
    <p class="approval-text approval-paragraph">
      {{ t('automationApproval.subjectsLead') }}
    </p>
    <details
      v-for="subject in review.subjects"
      :key="`${subject.principalId}:${subject.sessionId}`"
      class="approval-card"
      data-test="approval-subject"
    >
      <summary class="approval-text approval-summary">
        {{ subject.principalId }} · {{ subject.sessionId }}
      </summary>
      <p class="approval-text approval-paragraph">
        {{ t('automationApproval.grantRevision', { revision: subject.grantRevision }) }}
      </p>
      <p class="approval-text approval-paragraph">
        {{ t('automationApproval.configurationRevision', { revision: subject.configurationRevision }) }}
      </p>
      <BaseJsonView
        :value="subject.grant"
        max-height="28vh"
      />
    </details>
  </section>
</template>
