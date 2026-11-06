<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { LikeLeaderPayload } from '@/types.ts';
import type { ViewerIdentity } from '../lib/viewer-profile.ts';
import ViewerRankList from './ViewerRankList.vue';

const props = withDefaults(
  defineProps<{ leaders: readonly LikeLeaderPayload[]; images?: boolean }>(),
  { images: true },
);

const emit = defineEmits<{ select: [viewer: ViewerIdentity] }>();
const { t } = useI18n();
const entries = computed(() =>
  props.leaders.map((leader) => ({ ...leader, key: leader.userId || leader.uniqueId })),
);
</script>

<template>
  <ViewerRankList
    prefix="top-likes"
    :entries="entries"
    :caption="t('feed.viewers.sourceLikes')"
    :empty="t('feed.viewers.emptyLikes')"
    :images="images"
    @select="emit('select', $event)"
  />
</template>
