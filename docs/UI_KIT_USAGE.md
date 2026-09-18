# UI Kit — Quick Reference (canonical `src/web/components/ui/fields/`)

All primitives share the same controlled API + imperative `getValue/setValue` via `ref`.
FieldShell is the sole owner of label, required marker, info tooltip,
description/error, border, background, radius, focus ring, disabled state
and size. Never wrap a field in `FormField`/`FieldRow` — pass `label`,
`hint`, `description` and `error` straight to the field.

## Installation
Already imported in `src/web/styles.css` via `ui.css`. No new deps.

## Primitives

```vue
import { FieldShell, InputGroup } from './components/ui/fields/index.ts';
import { TextField, SearchField, NumberField, SelectField, PasswordField, TemplateField } from './components/ui/fields/index.ts';
import { Checkbox, Switch } from './components/ui/Checkbox.vue';
import { Card, Badge, Alert, EmptyState, Chip, ChipGroup } from './components/ui/Card.vue';
import { Button } from './components/ui/Button.vue';
import { DataTable, type Column } from './components/ui/Table.vue';
import { Page, PageHeader, SplitLayout, StatCard, StatGrid } from './components/ui/Page.vue';
import { IconSelect } from './components/ui/IconSelect.vue';
import { MultiSelect } from './components/ui/MultiSelect.vue';
```

`TextInput` / `NumberInput` / `Select` / `PasswordInput` / `SearchInput`
remain as thin compatibility shims over the canonical fields. New code
should import the `*Field` directly.

## Form — same `value/onValueChange` everywhere

```vue
// Text (prefix @, label owned by the field itself)
const userRef = useRef<TextFieldHandle>(null);
<TextField id="user" value={user} onValueChange={setUser} label="Usuario del Creador" hint="El @ es opcional" prefix="@" placeholder="handle" error={err} />
userRef.current?.getValue() // "crizthplay"
userRef.current?.setValue("other")
userRef.current?.clear()

// Number with stepper + suffix
<NumberField value={bonus} onValueChange={setBonus} label="Bonus" min={0} max={500} step={5} suffix="%" />

// Native select (OS popup; custom icon lists use IconSelect)
<SelectField value={locale} onValueChange={setLocale} label="Language" options={[{value:'en',label:'English'}]} />

// Icon select (custom listbox on the shared AutocompletePopover)
<IconSelect value={voice} onChange={setVoice} label="Voice" ariaLabel="Voice" options={voiceOptions} />

// ComboBox: TextField with options (recents, dynamic lists).
// Focus-empty opens, typing filters, pick commits — same popover,
// same field-anchored sizing as every autocomplete.
<TextField value={creator} onValueChange={setCreator} label="Creator" options={recentOptions} onOptionPick={connect} />

// Checkbox / Switch
<Checkbox checked={enabled} onCheckedChange={setEnabled} label="Puntos por Like" />
<Switch checked={enabled} onCheckedChange={setEnabled} label="Activo" />

// Search (with clear button)
<SearchField value={q} onValueChange={setQ} label="Search" placeholder="Buscar…" />
```

Dropdowns (`SelectField` native popup excluded) share one floating
primitive: `AutocompletePopover` + `autocomplete-list` classes. Field-
anchored popups match the anchor control width and clamp only to the
viewport/global maximum. Caret-anchored template autocomplete uses a
compact preferred width. Do not add per-input popup CSS.

## Card / Page

```vue
<Page narrow>
  <Card title="Conectar a TikTok LIVE" subtitle="..." icon={<IconRadio />}>
    ...
  </Card>
</Page>

<Page>
  <PageHeader title="Analíticas y Métricas" subtitle="..." icon={<IconBarChart/>} meta={<Badge>80 eventos</Badge>} />
  <StatGrid>
    <StatCard icon={<IconChat/>} value={10} label="Chats" tone="cyan" />
  </StatGrid>
  <DataTable ... />
</Page>

<SplitLayout left={<ConfigForm />} right={<LeaderboardTable />} />
```

## Table — one component for leaderboard + analytics ranking

```vue
const cols: Column<ViewerRecord>[] = [
  { key:'rank', header:'Puesto', width:'48px', render:(_r,i)=> <RankBadge rank={i+1}/> },
  { key:'viewer', header:'Espectador', render:(r)=> <span>@{r.uniqueId}</span> },
  { key:'level', header:'Nivel', width:'72px', align:'center', render:(r)=> <Badge tone="cyan">N.º {r.level}</Badge> },
  { key:'points', header:'Puntos', width:'88px', align:'right', render:(r)=> <span style={{color:'var(--tt-pink)',fontWeight:700}}>{r.points}</span> },
];

<DataTable
  columns={cols}
  data={filtered}
  rowKey="uniqueId"
  stickyHeader
  emptyState={<EmptyState title="Sin datos" description="Conecta a un LIVE..." />}
  rowClassName={(_,i)=> i<3 ? `top-rank-${i+1}` : undefined}
/>
```

## Button

```vue
<Button variant="primary" block>Conectar al LIVE</Button>
<Button variant="soft" size="sm" icon={<IconLock size={14} />} tooltip="Guest mode is fast & anonymous">Cookie de sesión</Button>
<Button variant="cyan" icon={<IconDice/>} iconOnly tooltip="Pick Random LIVE" />
<Button variant="danger" icon={<IconTrash/>} />
```

## Migration Checklist
- [ ] Replace `connect-card`/`tikfinity-card`/`stats-card-large` → `<Card>`
- [ ] Replace `input[type=text]`/`tikfinity-number-input`/`feed-search-wrap` → `TextField`/`NumberField`/`SearchField` with `label`/`hint`/`error` props directly (no `FormField` wrapper)
- [ ] Replace `tikfinity-toggle-row` → `<Checkbox/>` + `<NumberField/>` with direct labels
- [ ] Replace `tikfinity-table` + analytics flex list → `<DataTable>`
- [ ] Replace `.error-banner`/`.recent-chip`/inline styles → `<Alert>`/`<Chip>`/`<Badge>`
- [ ] Wrap each view in `<Page>` (`narrow` for Connect/Settings)

See [Development Guide](DEVELOPMENT.md) for frontend conventions, current view ownership, and the validation checklist.
