import { defineStore } from 'pinia';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { i18n } from '@/shared/i18n/i18n.ts';
import { createFeedHistory } from '../lib/feed-history.ts';
import { createLiveFeedState } from './live-feed-state.ts';

export const useLiveFeedStore = defineStore('live-feed', () =>
  createLiveFeedState(useControlClient(), {
    systemAuthor: () => i18n.global.t('feed.system'),
    discontinuityText: () => i18n.global.t('feed.resumed'),
    history: createFeedHistory(),
  }),
);
