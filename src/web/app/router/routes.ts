import type { RouteRecordRaw } from 'vue-router';
import { loadViewState } from '@/shared/lib/view-state.ts';
import type { AppTab } from '@/types.ts';
import { pluginPageRouteName, tabLocation } from './tab-routes.ts';

const developmentRoutes: RouteRecordRaw[] = import.meta.env.DEV
  ? [{ path: '/ui-kit', name: 'ui-kit', component: () => import('@/pages/ui-kit/ui/UiKitPage.vue') }]
  : [];

export const routes: RouteRecordRaw[] = [
  {
    path: '/',
    component: () => import('../layout/AppShell.vue'),
    children: [
      { path: 'debug', name: 'debug', component: () => import('@/pages/debug/ui/DebugPage.vue') },
      { path: '', name: 'home', redirect: () => tabLocation((loadViewState().tab as AppTab | undefined) ?? 'feed') },
      { path: 'feed', name: 'feed', meta: { tab: 'feed' }, component: () => import('@/pages/feed/ui/FeedPage.vue') },
      { path: 'points', name: 'points', meta: { tab: 'points' }, component: () => import('@/pages/points/ui/PointsPage.vue') },
      {
        path: 'analytics',
        name: 'analytics',
        meta: { tab: 'analytics' },
        component: () => import('@/pages/analytics/ui/AnalyticsPage.vue'),
      },
      {
        path: 'connections',
        name: 'connections',
        meta: { tab: 'connect' },
        component: () => import('@/pages/connections/ui/ConnectionsPage.vue'),
      },
      {
        path: 'automations',
        name: 'automations',
        meta: { tab: 'behavior' },
        component: () => import('@/pages/automations/ui/AutomationsPage.vue'),
      },
      { path: 'plugins', name: 'plugins', meta: { tab: 'plugins' }, component: () => import('@/pages/plugins/ui/PluginsPage.vue') },
      {
        path: 'plugins/:pluginId/:pageId',
        name: pluginPageRouteName,
        component: () => import('@/pages/plugin-page/ui/PluginPage.vue'),
      },
      {
        path: 'overlays',
        name: 'overlays',
        meta: { tab: 'widgets' },
        component: () => import('@/pages/overlays/ui/OverlaysPage.vue'),
      },
      { path: 'rankings', name: 'rankings', meta: { tab: 'rankings' }, component: () => import('@/pages/rankings/ui/RankingsPage.vue') },
      {
        path: 'settings',
        name: 'settings',
        meta: { tab: 'settings' },
        component: () => import('@/pages/settings/ui/SettingsPage.vue'),
      },
    ],
  },
  ...developmentRoutes,
  { path: '/:pathMatch(.*)*', redirect: '/' },
];
