<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { TopViewerPayload, ViewerRecord } from '@/types.ts';
import type { RankingSnapshot } from '../../../../widgets/shared/ranking-controller.ts';
import { rankedViewers } from '../lib/top-viewers.ts';
import type { ViewerIdentity } from '../lib/viewer-profile.ts';
import ViewerRankList from './ViewerRankList.vue';

const props = withDefaults(
  defineProps<{
    topViewers: readonly TopViewerPayload[];
    leaderboard: readonly ViewerRecord[];
    ranking?: RankingSnapshot | null;
    images?: boolean;
  }>(),
  { images: true, ranking: null },
);

const emit = defineEmits<{ select: [viewer: ViewerIdentity] }>();
const { t } = useI18n();
const entries = computed(() => rankedViewers(props.topViewers, props.leaderboard, props.ranking));
const SOURCE_LABELS = {
  ranking: 'feed.viewers.sourceRanking',
  tiktok_room: 'feed.viewers.sourceTikTok',
  loyalty_points: 'feed.viewers.sourcePoints',
} as const;
const caption = computed(() => (entries.value[0] ? t(SOURCE_LABELS[entries.value[0].provenance]) : ''));
</script>

<template>
  <ViewerRankList
    prefix="top-viewers"
    :entries="entries"
    :caption="caption"
    :empty="t('feed.viewers.empty')"
    :images="images"
    @select="emit('select', $event)"
  />
</template>
