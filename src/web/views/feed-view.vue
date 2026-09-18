<script lang="tsx">
import type { VNode } from 'vue';

import { EventCard } from '../components/event-card.vue';
import { IconArrowDown, IconBolt, IconChat, IconDot, IconGift, IconHeart, IconPause, IconRadio, IconTrash, IconUsers } from '../components/icons.vue';
import { Button } from '../components/ui/Button.vue';
import { SearchInput } from '../components/ui/TextInput.vue';
import { Tooltip } from '../components/ui/Tooltip.vue';
import { defineVueComponent } from '../vue/component.ts';
import { t, type Locale } from '../i18n.ts';
import type { DisplayEvent, EventFilter, TopViewerPayload, ViewerRecord } from '../types.ts';
import { TopViewersRibbon } from '../components/top-viewers.vue';

type FeedViewProps = {
  locale: Locale;
  events: DisplayEvent[];
  leaderboard?: ViewerRecord[];
  topViewers?: TopViewerPayload[];
  liveViewers?: number;
  filter: EventFilter;
  searchQuery: string;
  autoScroll: boolean;
  unreadCount: number;
  onFilterChange: (f: EventFilter) => void;
  onSearchChange: (q: string) => void;
  onToggleAutoScroll: () => void;
  onClearFeed: () => void;
  streamContainerRef: (el: Element | null) => void;
};

function renderFeedView({
  locale,
  events,
  leaderboard = [],
  topViewers = [],
  liveViewers = 0,
  filter,
  searchQuery,
  autoScroll,
  unreadCount,
  onFilterChange,
  onSearchChange,
  onToggleAutoScroll,
  onClearFeed,
  streamContainerRef,
}: FeedViewProps) {
  const filterButtons: Array<{ key: EventFilter; tooltip: string; icon: VNode }> = [
    { key: 'all', tooltip: t(locale, 'filterAll'), icon: <IconBolt /> },
    { key: 'chat', tooltip: t(locale, 'filterChats'), icon: <IconChat /> },
    { key: 'gift', tooltip: t(locale, 'filterGifts'), icon: <IconGift /> },
    { key: 'like', tooltip: t(locale, 'filterLikes'), icon: <IconHeart /> },
    { key: 'social', tooltip: t(locale, 'filterSocial'), icon: <IconUsers /> },
  ];

  const filteredEvents = events.filter((ev) => {
    if (filter !== 'all') {
      if (filter === 'chat' && ev.kind !== 'chat') return false;
      if (filter === 'gift' && ev.kind !== 'gift') return false;
      if (filter === 'like' && ev.kind !== 'like') return false;
      if (filter === 'social' && ev.kind !== 'social' && ev.kind !== 'member') return false;
    }
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      return ev.author.toLowerCase().includes(q) || ev.text.toLowerCase().includes(q);
    }
    return true;
  });

  return (
    <main class="feed-pane">
      <div class="feed-toolbar">
        <div class="filter-icon-group" role="tablist" aria-label={t(locale, 'filterAll')}>
          {filterButtons.map((btn) => (
            <Tooltip key={btn.key} text={btn.tooltip} position="bottom">
              <button
                type="button"
                class={`filter-icon-btn ${filter === btn.key ? 'active' : ''}`}
                aria-label={btn.tooltip}
                aria-pressed={filter === btn.key}
                onClick={() => onFilterChange(btn.key)}
              >
                {btn.icon}
              </button>
            </Tooltip>
          ))}
        </div>

        <SearchInput value={searchQuery} onValueChange={onSearchChange} placeholder={t(locale, 'searchEvents')} />
      </div>

      <div class="feed-layout">
        <section class="feed-main">
          <div class="feed-stream" ref={(element) => streamContainerRef(element as Element | null)}>
            {filteredEvents.length === 0 ? (
              <div class="feed-empty">
                <div class="empty-icon empty-icon--illustration">
                  <IconRadio size={28} />
                </div>
                <p>{t(locale, 'messagesEmpty')}</p>
              </div>
            ) : (
              filteredEvents.map((ev) => <EventCard key={ev.id} event={ev} locale={locale} />)
            )}
          </div>

          <footer class="feed-footer-info">
            <span>
              {t(locale, filteredEvents.length === 1 ? 'messageCountOne' : 'messageCountMany', {
                count: filteredEvents.length,
              })}
            </span>
            <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
              <Button
                size="sm"
                variant="soft"
                tooltip={autoScroll ? t(locale, 'autoScrollOn') : t(locale, 'autoScrollPaused')}
                icon={autoScroll ? <span style={{ color: 'var(--tt-green)', display: 'inline-flex' }}><IconDot /></span> : <IconPause />}
                iconOnly
                onClick={onToggleAutoScroll}
              />
              <Button size="sm" variant="soft" tooltip={t(locale, 'clearFeed')} icon={<IconTrash />} iconOnly onClick={onClearFeed} />
            </div>
          </footer>
        </section>

        <aside class="feed-side" aria-label={t(locale, 'topContributors')}>
          <TopViewersRibbon locale={locale} topViewers={topViewers} leaderboard={leaderboard} liveViewers={liveViewers} />
        </aside>
      </div>

      {!autoScroll && unreadCount > 0 ? (
        <div class="feed-floating-bar">
          <Button variant="primary" size="sm" icon={<IconArrowDown />} onClick={onToggleAutoScroll}>
            {t(locale, 'scrollToBottom', { count: unreadCount })}
          </Button>
        </div>
      ) : null}
    </main>
  );
}

export const FeedView = defineVueComponent<FeedViewProps>(
  [
    'locale',
    'events',
    'leaderboard',
    'topViewers',
    'liveViewers',
    'filter',
    'searchQuery',
    'autoScroll',
    'unreadCount',
    'onFilterChange',
    'onSearchChange',
    'onToggleAutoScroll',
    'onClearFeed',
    'streamContainerRef',
  ],
  (props) => () => renderFeedView(props),
);

export default FeedView;
</script>
