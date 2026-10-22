<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BaseFormGrid from '@/shared/ui/form/BaseFormGrid.vue';
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';

const props = defineProps<{
  pluginName: string;
  working: boolean;
  result?: { ok: boolean; message: string };
  simple?: boolean;
}>();

const emit = defineEmits<{ submit: [username: string, password: string] }>();
const open = defineModel<boolean>('open', { default: false });
const { t } = useI18n();
const username = ref(props.simple ? 'admin' : '');
const password = ref('');
const attempted = ref(false);

const succeeded = computed(() => attempted.value && !props.working && props.result?.ok === true);
const failed = computed(() => attempted.value && !props.working && props.result !== undefined && !props.result.ok);
const canSubmit = computed(() => !props.working && username.value.trim().length > 0 && password.value.length > 0);

watch(open, (isOpen) => {
  if (isOpen) return;
  attempted.value = false;
  password.value = '';
});

function submit(): void {
  if (!canSubmit.value) return;
  attempted.value = true;
  emit('submit', username.value.trim(), password.value);
  password.value = '';
}
</script>

<template>
  <BaseDialog
    v-model:open="open"
    :title="t(simple ? 'plugins.connection.authorize' : 'plugins.provision.title')"
    :closable="!working"
    :width="440"
  >
    <p class="provision__lead">
      {{ t(simple ? 'plugins.connection.signInHint' : 'plugins.provision.lead', { name: pluginName }) }}
    </p>
    <p
      v-if="succeeded || failed"
      class="provision__result"
      :class="{ 'provision__result--ok': succeeded }"
      role="status"
    >
      {{ simple && succeeded ? t('plugins.connection.saved') : result?.message }}
    </p>
    <form
      v-if="!succeeded"
      @submit.prevent="submit"
    >
      <BaseFormGrid>
        <BaseFormRow
          for="provision-user"
          :label="t('plugins.provision.adminId')"
        >
          <BaseTextField
            id="provision-user"
            v-model="username"
            autocomplete="username"
            :disabled="working"
          />
        </BaseFormRow>
        <BaseFormRow
          for="provision-password"
          :label="t('plugins.provision.adminPassword')"
        >
          <BaseTextField
            id="provision-password"
            v-model="password"
            type="password"
            autocomplete="current-password"
            :disabled="working"
            @keydown.enter="submit"
          />
        </BaseFormRow>
      </BaseFormGrid>
    </form>
    <template #footer>
      <BaseButton
        v-if="succeeded"
        variant="primary"
        @click="open = false"
      >
        {{ t('common.close') }}
      </BaseButton>
      <template v-else>
        <BaseButton
          variant="primary"
          :disabled="!canSubmit"
          @click="submit"
        >
          {{ working ? t('plugins.provision.working') : t(simple ? 'plugins.connection.connectSonicBoom' : 'plugins.provision.submit') }}
        </BaseButton>
        <BaseButton
          :disabled="working"
          @click="open = false"
        >
          {{ t('common.cancel') }}
        </BaseButton>
      </template>
    </template>
  </BaseDialog>
</template>

<style scoped>
.provision__lead {
  margin: 0 0 var(--space-5);
  color: var(--color-text-muted);
}

.provision__result {
  margin: 0 0 var(--space-5);
  padding: var(--space-3) var(--space-4);
  border: var(--border-width) solid var(--color-danger);
  border-radius: var(--radius-sm);
  color: var(--color-danger-hover);
}

.provision__result--ok {
  border-color: var(--color-success);
  color: var(--color-success);
}
</style>
