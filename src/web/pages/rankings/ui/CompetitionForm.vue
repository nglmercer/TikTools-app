<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { CompetitionDefinition } from '@/entities/competition/types.ts';
import type { RankingDefinition } from '@/entities/ranking/types.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import { blankOption, competitionTypes, draftFromOption, invalidInputs, toOption, type CompetitionType } from '../lib/competition-draft.ts';
import CompetitionOptionRow from './CompetitionOptionRow.vue';
import FormField from './FormField.vue';
import FormSection from './FormSection.vue';

const props = defineProps<{ rankings: RankingDefinition[]; actions: { id: string; name: string }[]; editing: CompetitionDefinition | null }>();
const emit = defineEmits<{ saved: [id: string, updated: boolean]; cancel: [] }>();
const { t } = useI18n();
const client = useControlClient();

const initial = props.editing;
const type = ref<CompetitionType>(initial ? (initial.kind === 'weighted_poll' ? 'poll' : initial.kind) : 'poll');
const id = ref(initial?.id ?? '');
const name = ref(initial?.name ?? '');
const rankingId = ref(initial?.rankingId ?? '');
const duration = ref((initial?.durationMs ?? 0) / 1000);
const voteLimit = ref(initial?.voteLimit ?? 1);
const allowSwitch = ref(initial?.allowSwitch ?? false);
const minimumParticipants = ref(initial?.minimumParticipants ?? 1);
const tiePolicy = ref<CompetitionDefinition['tiePolicy']>(initial?.tiePolicy ?? 'stable');
const mode = ref<CompetitionDefinition['selectionMode']>(initial?.selectionMode ?? 'highest_score');
const targetScore = ref(initial?.targetScore ?? 0);
const userActionId = ref(initial?.onUserWin?.actionId ?? '');
const options = ref(initial ? initial.options.map(draftFromOption) : [blankOption(), blankOption()]);
const busy = ref(false);
const error = ref('');

const challenge = computed(() => type.value === 'leaderboard_challenge');
const draw = computed(() => type.value === 'random_draw' || type.value === 'weighted_draw');
const userWinner = computed(() => challenge.value || draw.value);
const eligibleRankings = computed(() => challenge.value
  ? props.rankings.filter((r) => !r.sources.some((s) => s.source === 'competition.vote'))
  : props.rankings.filter((r) => r.groupBy === 'option' && r.window.type === 'session' && r.sources.some((s) => s.source === 'competition.vote' && s.metric === 'event.count') && r.sources.every((s) => ['competition.vote', 'tiktok.like', 'tiktok.share', 'tiktok.gift'].includes(s.source))));
const validDuration = () => Number.isSafeInteger(duration.value * 1000) && duration.value >= 0 && duration.value <= 86400000;
const positive = (value: number) => Number.isSafeInteger(value) && value >= 1;

function validationError(): string {
  if (options.value.some(invalidInputs)) return t('competitionsPanel.form.invalidInputs');
  const basics = id.value.trim() && name.value.trim() && eligibleRankings.value.some((ranking) => ranking.id === rankingId.value) && positive(minimumParticipants.value) && validDuration();
  if (challenge.value) return basics && Number.isSafeInteger(targetScore.value) && targetScore.value >= 0 ? '' : t('competitionsPanel.form.invalidChallenge');
  const cleaned = options.value.map(toOption);
  const commands = cleaned.flatMap((option) => option.commands.map((command) => command.toLowerCase()));
  const gifts = cleaned.flatMap((option) => option.giftIds);
  const valid = basics && positive(voteLimit.value) && cleaned.length >= 1
    && cleaned.every((option) => option.id && option.label && (option.commands.length || option.giftIds.length) && option.commands.length <= 64 && option.commands.every((command) => /^!\S+$/.test(command))
      && option.giftIds.length <= 256 && option.giftIds.every((gift) => /^[1-9][0-9]{0,19}$/.test(gift) && BigInt(gift) <= 18446744073709551615n))
    && new Set(cleaned.map((option) => option.id)).size === cleaned.length && new Set(commands).size === commands.length && new Set(gifts).size === gifts.length;
  return valid ? '' : t('competitionsPanel.form.invalid');
}

function kind(): CompetitionDefinition['kind'] {
  if (type.value !== 'poll') return type.value;
  return voteLimit.value === 1 && initial?.kind !== 'weighted_poll' ? 'poll' : 'weighted_poll';
}

async function submit(): Promise<void> {
  error.value = validationError();
  if (error.value) return;
  const userAction = userWinner.value && userActionId.value ? { actionId: userActionId.value, inputs: initial?.onUserWin?.actionId === userActionId.value ? initial.onUserWin.inputs : {} } : null;
  const definition: CompetitionDefinition = {
    schemaVersion: 1, id: id.value.trim(), name: name.value.trim(), kind: kind(), rankingId: rankingId.value,
    options: challenge.value ? [] : options.value.map(toOption),
    selectionMode: type.value === 'weighted_draw' ? 'weighted_random' : type.value === 'random_draw' ? 'uniform_random' : challenge.value ? 'highest_score' : mode.value,
    tiePolicy: tiePolicy.value, durationMs: duration.value ? duration.value * 1000 : null, voteLimit: voteLimit.value, allowSwitch: allowSwitch.value, minimumParticipants: minimumParticipants.value,
    targetScore: challenge.value && targetScore.value > 0 ? targetScore.value : null, onUserWin: userAction,
  };
  busy.value = true;
  try {
    await client.call(initial ? 'competitions.update' : 'competitions.create', { ...definition });
    emit('saved', definition.id, !!initial);
  } catch (failure) { error.value = errorMessage(failure); } finally { busy.value = false; }
}
</script>

<template>
  <BasePanel :title="editing ? t('competitionsPanel.form.editTitle') : t('competitionsPanel.form.createTitle')">
    <form
      class="competition-form"
      @submit.prevent="submit"
    >
      <p
        v-if="error"
        class="competition-form__error"
        role="alert"
      >
        {{ error }}
      </p>
      <FormSection :title="t('competitionsPanel.form.step2')">
        <div class="row">
          <FormField :label="t('competitionsPanel.form.type')">
            <select
              v-model="type"
              data-field="type"
            >
              <option
                v-for="value in competitionTypes"
                :key="value"
                :value="value"
              >
                {{ t(`competitionsPanel.types.${value}.name`) }}
              </option>
            </select>
          </FormField>
          <FormField :label="t('competitionsPanel.form.name')">
            <input
              v-model="name"
              data-field="name"
              required
              maxlength="256"
            >
          </FormField>
          <FormField :label="t('competitionsPanel.form.id')">
            <input
              v-model="id"
              data-field="id"
              :disabled="!!editing"
              required
              maxlength="128"
            >
          </FormField>
          <FormField :label="challenge ? t('competitionsPanel.form.ranking') : t('competitionsPanel.form.voteRanking')">
            <select
              v-model="rankingId"
              data-field="ranking"
            >
              <option value="">
                {{ t('competitionsPanel.form.chooseRanking') }}
              </option>
              <option
                v-for="ranking in eligibleRankings"
                :key="ranking.id"
                :value="ranking.id"
              >
                {{ ranking.id }}
              </option>
            </select>
          </FormField>
        </div>
        <p class="hint">
          {{ t(`competitionsPanel.types.${type}.hint`) }}
        </p>
      </FormSection>

      <FormSection :title="t('competitionsPanel.form.step3')">
        <div class="row">
          <FormField :label="t('competitionsPanel.form.duration')">
            <input
              v-model.number="duration"
              type="number"
              min="0"
              max="86400000"
            >
          </FormField>
          <FormField
            v-if="!challenge"
            :label="t('competitionsPanel.form.voteLimit')"
          >
            <input
              v-model.number="voteLimit"
              type="number"
              min="1"
            >
          </FormField>
          <FormField :label="t('competitionsPanel.form.minimumParticipants')">
            <input
              v-model.number="minimumParticipants"
              type="number"
              min="1"
              max="100000"
            >
          </FormField>
          <FormField
            v-if="challenge"
            :label="t('competitionsPanel.form.targetScore')"
          >
            <input
              v-model.number="targetScore"
              type="number"
              min="0"
              step="1"
            >
          </FormField>
        </div>
        <div class="row">
          <FormField
            v-if="!challenge && !draw"
            :label="t('competitionsPanel.form.selection')"
          >
            <select v-model="mode">
              <option
                v-for="value in (['highest_score', 'uniform_random', 'weighted_random'] as const)"
                :key="value"
                :value="value"
              >
                {{ t(`competitionsPanel.form.selectionModes.${value}`) }}
              </option>
            </select>
          </FormField>
          <FormField
            v-if="!draw && (challenge || mode === 'highest_score')"
            :label="t('competitionsPanel.form.tiePolicy')"
          >
            <select v-model="tiePolicy">
              <option
                v-for="value in (['stable', 'random', 'manual', 'no_winner'] as const)"
                :key="value"
                :value="value"
              >
                {{ t(`competitionsPanel.form.tiePolicies.${value}`) }}
              </option>
            </select>
          </FormField>
          <FormField
            v-if="userWinner"
            :label="t('competitionsPanel.form.userAction')"
          >
            <select v-model="userActionId">
              <option value="">
                {{ t('competitionsPanel.form.noAction') }}
              </option>
              <option
                v-for="available in actions"
                :key="available.id"
                :value="available.id"
              >
                {{ available.name || available.id }}
              </option>
            </select>
          </FormField>
        </div>
        <BaseCheckbox
          v-if="!challenge"
          v-model="allowSwitch"
        >
          {{ t('competitionsPanel.form.allowSwitch') }}
        </BaseCheckbox>
      </FormSection>

      <FormSection
        v-if="!challenge"
        :title="t('competitionsPanel.form.step4')"
      >
        <CompetitionOptionRow
          v-for="(_, index) in options"
          :key="index"
          v-model="options[index]!"
          :actions="actions"
          :removable="options.length > 1"
          :with-action="!draw"
          @remove="options.splice(index, 1)"
        />
        <BaseButton
          class="add-option"
          :disabled="options.length >= 256"
          @click="options.push(blankOption())"
        >
          {{ t('competitionsPanel.form.addOption') }}
        </BaseButton>
      </FormSection>

      <div class="actions">
        <BaseButton
          type="submit"
          variant="primary"
          :disabled="busy"
        >
          {{ editing ? t('competitionsPanel.form.save') : t('competitionsPanel.form.create') }}
        </BaseButton>
        <BaseButton @click="emit('cancel')">
          {{ t('competitionsPanel.form.cancel') }}
        </BaseButton>
      </div>
    </form>
  </BasePanel>
</template>

<style scoped>
.competition-form { display: grid; gap: var(--space-4); }
.competition-form__error { margin: 0; color: var(--color-danger, #fb7185); }
.row { display: flex; flex-wrap: wrap; align-items: end; gap: var(--space-3); }
.hint { margin: 0; font-size: 12px; color: var(--color-text-muted); }
.add-option { justify-self: start; }
.actions { display: flex; gap: var(--space-2); }
</style>
