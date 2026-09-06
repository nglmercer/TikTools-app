<script lang="tsx">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { defineVueComponent } from '../vue/component.ts';

import { IconBolt, IconCheck, IconCoins, IconFlame, IconStar, IconTrash, IconTrophy } from '../components/icons.vue';
import { Alert, Badge, Card, EmptyState } from '../components/ui/Card.vue';
import { Button } from '../components/ui/Button.vue';
import { Checkbox } from '../components/ui/Checkbox.vue';
import { FieldRow, FormField } from '../components/ui/FormField.vue';
import { NumberInput } from '../components/ui/NumberInput.vue';
import { SearchInput, TextInput } from '../components/ui/TextInput.vue';
import { SplitLayout } from '../components/ui/Page.vue';
import { DataTable, RowActions, type Column } from '../components/ui/Table.vue';
import { t, type Locale } from '../i18n.ts';
import type { ConnectionStatus, PointsConfig, ViewerRecord } from '../types.ts';
import { useDialogs } from '../composables/useDialogs.ts';

type PointsViewProps = {
  locale: Locale;
  config: PointsConfig;
  leaderboard: ViewerRecord[];
  status?: ConnectionStatus;
  onUpdateConfig: (updated: Partial<PointsConfig>) => void;
  onResetPoints: (uniqueId?: string) => void;
  onAdjustPoints: (uniqueId: string, delta: number) => void;
};

export const PointsView = defineVueComponent<PointsViewProps>(
  ['locale', 'config', 'leaderboard', 'status', 'onUpdateConfig', 'onResetPoints', 'onAdjustPoints'],
  (props) => {
  const localConfig = ref<PointsConfig>(props.config);
  const searchQuery = ref('');
  const saveSuccess = ref(false);
  const adjustTarget = ref<string | null>(null);
  const adjustDelta = ref('50');
  const deductMode = ref(false);
  const page = ref(1);
  const pageSize = ref(10);
  const sortBy = ref<string>('points');
  const sortDir = ref<'asc' | 'desc'>('desc');
  const leaderboardWrapRef = ref<HTMLDivElement | null>(null);
  const dialogs = useDialogs();

  const isLive = computed(() => props.status === 'connected' || props.status === 'connecting' || props.status === 'retrying');

  watch(() => props.config, (config) => { localConfig.value = config; });

  // Auto-calc pageSize to fill available height and avoid empty gap (Image 1)
  onMounted(() => {
    const el = leaderboardWrapRef.value;
    if (!el) return;
    const ROW_H = 37; // td + border
    const compute = () => {
      // only auto on desktop where split is 2-col
      if (window.innerWidth <= 960) return;
      const rect = el.getBoundingClientRect();
      // available height = wrapper height; subtract search (~46) + pagination (~38)
      const avail = rect.height - 46 - 38 - 16;
      const rows = Math.floor(avail / ROW_H);
      const auto = Math.max(10, Math.min(50, rows > 0 ? rows : 10));
      // snap to 5 step to avoid jitter, keep minimal that fills
      const snapped = Math.ceil(auto / 5) * 5;
      if (Math.abs(pageSize.value - snapped) > 2) pageSize.value = snapped;
    };
    const ro = new ResizeObserver(compute);
    ro.observe(el);
    window.addEventListener('resize', compute);
    // initial tick after layout
    requestAnimationFrame(compute);
    onUnmounted(() => {
      ro.disconnect();
      window.removeEventListener('resize', compute);
    });
  });
  // reset page when search changes
  watch([searchQuery, pageSize], () => { page.value = 1; });

  const handleSave = (e: SubmitEvent) => {
    e.preventDefault();
    if (isLive.value) return;
    props.onUpdateConfig(localConfig.value);
    saveSuccess.value = true;
    setTimeout(() => { saveSuccess.value = false; }, 3000);
  };

  const handleResetAll = async () => {
    const confirmed = await dialogs.confirm(t(props.locale, 'resetPointsConfirm'), {
      title: t(props.locale, 'resetPoints'),
      confirmLabel: t(props.locale, 'resetPoints'),
      cancelLabel: t(props.locale, 'cancel'),
      danger: true,
    });
    if (confirmed) props.onResetPoints();
  };

  const handleResetViewer = async (uniqueId: string) => {
    const confirmed = await dialogs.confirm(t(props.locale, 'resetViewerConfirm', { uniqueId }), {
      title: t(props.locale, 'resetPoints'),
      confirmLabel: t(props.locale, 'resetPoints'),
      cancelLabel: t(props.locale, 'cancel'),
      danger: true,
    });
    if (confirmed) props.onResetPoints(uniqueId);
  };

  const handleAdjustSubmit = (e: SubmitEvent) => {
    e.preventDefault();
    if (!adjustTarget.value) return;
    const base = parseFloat(adjustDelta.value);
    if (Number.isNaN(base)) return;
    const delta = deductMode.value ? -Math.abs(base) : Math.abs(base);
    props.onAdjustPoints(adjustTarget.value, delta);
    adjustTarget.value = null;
  };

  const filteredViewers = computed(() => {
    let out = props.leaderboard.filter((v) => {
      if (!searchQuery.value.trim()) return true;
      const q = searchQuery.value.toLowerCase().replace(/^@/, '');
      return v.uniqueId.toLowerCase().includes(q) || (v.nickname && v.nickname.toLowerCase().includes(q));
    });
    // sorting
    out = [...out].sort((a, b) => {
      if (sortBy.value === 'points') return sortDir.value === 'asc' ? a.points - b.points : b.points - a.points;
      if (sortBy.value === 'level') return sortDir.value === 'asc' ? a.level - b.level : b.level - a.level;
      if (sortBy.value === 'viewer') return sortDir.value === 'asc' ? a.uniqueId.localeCompare(b.uniqueId) : b.uniqueId.localeCompare(a.uniqueId);
      return 0;
    });
    return out;
  });

  const openAdd = (id: string) => { adjustTarget.value = id; adjustDelta.value = '50'; deductMode.value = false; };
  const openDeduct = (id: string) => { adjustTarget.value = id; adjustDelta.value = '50'; deductMode.value = true; };
  const patchConfig = <K extends keyof PointsConfig>(key: K, value: PointsConfig[K]): void => {
    localConfig.value = { ...localConfig.value, [key]: value };
  };

  return () => {
  const locale = props.locale;
  const config = localConfig.value;
  const live = isLive.value;

  const columns: Column<ViewerRecord>[] = [
    {
      key: 'rank',
      header: t(locale, 'rank'),
      width: '56px',
      render: (_row, idx) =>
        idx === 0 ? (
          <span style={{ color: '#b45309', fontWeight: 800, display: 'inline-flex', alignItems: 'center', gap: 4 }}>
            <IconTrophy /> 1
          </span>
        ) : idx === 1 ? (
          <span style={{ color: '#64748b', fontWeight: 800, display: 'inline-flex', alignItems: 'center', gap: 4 }}>
            <IconTrophy /> 2
          </span>
        ) : idx === 2 ? (
          <span style={{ color: '#92400e', fontWeight: 800, display: 'inline-flex', alignItems: 'center', gap: 4 }}>
            <IconTrophy /> 3
          </span>
        ) : (
          <span style={{ color: 'var(--text-muted)', fontSize: 11 }}>#{idx + 1}</span>
        ),
    },
    {
      key: 'viewer',
      header: t(locale, 'viewer'),
      sortable: true,
      render: (row) => (
        <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
          <span style={{ fontWeight: 600 }}>@{row.uniqueId}</span>
          {row.isSubscriber ? (
            <span title="Subscriber" style={{ display: 'inline-flex', color: '#f59e0b' }}>
              <IconStar />
            </span>
          ) : null}
        </span>
      ),
    },
    {
      key: 'level',
      header: t(locale, 'level'),
      width: '84px',
      sortable: true,
      render: (row) => (
        <span class="tt-badge-level" style={{ transform: 'scale(0.88)', transformOrigin: 'left' }}>
          <span class="tt-badge-icon">
            <IconBolt />
          </span>
          <span class="tt-badge-text">N.º {row.level}</span>
        </span>
      ),
    },
    {
      key: 'points',
      header: t(locale, 'points'),
      width: '88px',
      align: 'right',
      sortable: true,
      render: (row) => <span style={{ fontWeight: 700, color: 'var(--tt-pink)' }}>{row.points.toLocaleString()}</span>,
    },
    {
      key: 'actions',
      header: t(locale, 'actions'),
      width: '84px',
      align: 'right',
      render: (row) => (
        <div style={{ display: 'inline-flex', gap: 4, alignItems: 'center' }}>
          <Button size="sm" variant="soft" tooltip={t(locale, 'addPoints')} onClick={() => openAdd(row.uniqueId)}>
            +
          </Button>
          <RowActions onAdd={() => openAdd(row.uniqueId)} onDeduct={() => openDeduct(row.uniqueId)} onReset={() => { void handleResetViewer(row.uniqueId); }} />
        </div>
      ),
    },
  ];

  return (
    <div class="view-container" style={{ flexDirection: 'column' }}>
      <SplitLayout
        left={
          <form onSubmit={handleSave} style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            {live ? <Alert variant="info">LIVE {t(locale, 'live')} — {t(locale, 'configLockedLive')}</Alert> : null}
            <Card title={t(locale, 'pointsSystem')} icon={<IconCoins />}>
              <TextInput
                id="tf-currency-name"
                name="currencyName"
                value={config.currencyName}
                onValueChange={(v) => { localConfig.value = { ...localConfig.value, currencyName: v }; }}
                label={t(locale, 'currencyName')}
                disabled={live}
              />

              <FieldRow label={t(locale, 'pointsPerCoin')}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Checkbox
                    name="pointsPerCoinEnabled"
                    checked={config.pointsPerCoinEnabled}
                    onCheckedChange={(v) => !live && patchConfig('pointsPerCoinEnabled', v)}
                    disabled={live}
                  />
                  <NumberInput
                    name="pointsPerCoin"
                    value={config.pointsPerCoin}
                    onValueChange={(v) => patchConfig('pointsPerCoin', v ?? 0)}
                    min={0}
                    step={0.1}
                    disabled={live || !config.pointsPerCoinEnabled}
                  />
                </div>
              </FieldRow>

              <FieldRow label={t(locale, 'pointsPerShare')}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Checkbox
                    name="pointsPerShareEnabled"
                    checked={config.pointsPerShareEnabled}
                    onCheckedChange={(v) => !live && patchConfig('pointsPerShareEnabled', v)}
                    disabled={live}
                  />
                  <NumberInput
                    name="pointsPerShare"
                    value={config.pointsPerShare}
                    onValueChange={(v) => patchConfig('pointsPerShare', v ?? 0)}
                    min={0}
                    step={0.5}
                    disabled={live || !config.pointsPerShareEnabled}
                  />
                </div>
              </FieldRow>

              <FieldRow label={t(locale, 'pointsPerChat')}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Checkbox
                    name="pointsPerChatEnabled"
                    checked={config.pointsPerChatEnabled}
                    onCheckedChange={(v) => !live && patchConfig('pointsPerChatEnabled', v)}
                    disabled={live}
                  />
                  <NumberInput
                    name="pointsPerChat"
                    value={config.pointsPerChat}
                    onValueChange={(v) => patchConfig('pointsPerChat', v ?? 0)}
                    min={0}
                    step={0.1}
                    disabled={live || !config.pointsPerChatEnabled}
                  />
                </div>
              </FieldRow>

              <FieldRow label={t(locale, 'pointsPerLike')}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Checkbox
                    name="pointsPerLikeEnabled"
                    checked={config.pointsPerLikeEnabled}
                    onCheckedChange={(v) => !live && patchConfig('pointsPerLikeEnabled', v)}
                    disabled={live}
                  />
                  <NumberInput
                    name="pointsPerLike"
                    value={config.pointsPerLike}
                    onValueChange={(v) => patchConfig('pointsPerLike', v ?? 0)}
                    min={0}
                    step={0.05}
                    disabled={live || !config.pointsPerLikeEnabled}
                  />
                </div>
              </FieldRow>

              <FieldRow label={t(locale, 'pointsPerFollow')}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Checkbox
                    name="pointsPerFollowEnabled"
                    checked={config.pointsPerFollowEnabled}
                    onCheckedChange={(v) => !live && patchConfig('pointsPerFollowEnabled', v)}
                    disabled={live}
                  />
                  <NumberInput
                    name="pointsPerFollow"
                    value={config.pointsPerFollow}
                    onValueChange={(v) => patchConfig('pointsPerFollow', v ?? 0)}
                    min={0}
                    step={1}
                    disabled={live || !config.pointsPerFollowEnabled}
                  />
                </div>
              </FieldRow>
            </Card>

            <Card title={t(locale, 'subBonus')} subtitle={t(locale, 'subBonusLead')} icon={<IconStar />}>
              <FieldRow label={t(locale, 'subBonusRatio')}>
                <NumberInput
                  name="subBonusMultiplier"
                  value={config.subBonusMultiplier}
                  onValueChange={(v) => patchConfig('subBonusMultiplier', v ?? 0)}
                  min={0}
                  max={500}
                  step={5}
                  suffix="%"
                  disabled={live}
                />
              </FieldRow>
            </Card>

            <Card title={t(locale, 'levelConfig')} subtitle={t(locale, 'levelConfigLead')} icon={<IconFlame />}>
              <FieldRow label={t(locale, 'pointsPerLevel')}>
                <NumberInput
                  name="pointsPerLevel"
                  value={config.pointsPerLevel}
                  onValueChange={(v) => patchConfig('pointsPerLevel', Math.max(10, (v ?? 10) | 0))}
                  min={10}
                  step={10}
                  disabled={live}
                />
              </FieldRow>
            </Card>

            <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
              <Button type="submit" variant="primary" block icon={<IconCheck />} disabled={live}>
                {t(locale, 'savePointsConfig')}
              </Button>
              <Button variant="danger" icon={<IconTrash />} tooltip={t(locale, 'resetPoints')} onClick={handleResetAll} iconOnly disabled={live} />
            </div>

            {saveSuccess.value ? (
              <Alert variant="success">
                <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
                  <IconCheck /> {t(locale, 'configSaved')}
                </span>
              </Alert>
            ) : null}
          </form>
        }
        right={
          <div ref={leaderboardWrapRef} style={{ display: 'flex', flexDirection: 'column', minHeight: 0, height: '100%' }}>
            <Card title={t(locale, 'leaderboard')} icon={<IconTrophy />} action={<Badge>{filteredViewers.value.length} {t(locale, 'viewersCount')}</Badge>} padding="md" class="ui-card--fill">
              <div style={{ marginBottom: 10 }}>
                <SearchInput value={searchQuery.value} onValueChange={(value) => { searchQuery.value = value; }} placeholder={t(locale, 'searchViewers')} />
              </div>
              <DataTable
                columns={columns}
                data={filteredViewers.value}
                rowKey="uniqueId"
                emptyState={<EmptyState title={t(locale, 'noData')} />}
                rowClassName={(_r, i) => (i < 3 ? `top-rank-${i + 1}` : undefined)}
                pagination={{ page: page.value, pageSize: pageSize.value, total: filteredViewers.value.length, onPageChange: (value) => { page.value = value; }, onPageSizeChange: (value) => { pageSize.value = value; page.value = 1; }, pageSizeOptions: [10, 15, 20, 30, 50] }}
                sortBy={sortBy.value}
                sortDir={sortDir.value}
                onSortChange={(key, direction) => { sortBy.value = key; sortDir.value = direction; }}
              />
            </Card>
          </div>
        }
      />

      {adjustTarget.value ? (
        <div class="modal-backdrop">
          <div class="modal-card">
            <h2 style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <IconCoins /> {deductMode.value ? 'Deduct' : 'Add'} Points: @{adjustTarget.value}
            </h2>
            <form onSubmit={handleAdjustSubmit}>
              <FormField label={deductMode.value ? 'Points to deduct:' : 'Points to add:'}>
                <NumberInput value={parseFloat(adjustDelta.value) || 0} onValueChange={(value) => { adjustDelta.value = String(Math.abs(value ?? 0)); }} min={0} step={1} />
              </FormField>
              <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end', marginTop: 12 }}>
                <Button variant="soft" onClick={() => { adjustTarget.value = null; }}>
                  {t(locale, 'cancel')}
                </Button>
                <Button type="submit" variant={deductMode.value ? 'danger' : 'primary'}>
                  {deductMode.value ? 'Deduct' : t(locale, 'continue')}
                </Button>
              </div>
            </form>
          </div>
        </div>
      ) : null}
    </div>
  );
  };
  },
);

export default PointsView;
</script>
