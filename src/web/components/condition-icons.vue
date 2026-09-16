<script lang="tsx">
import type { FieldIcon } from '../../automation/behavior/fields.ts';
import type { FilterOperator, I18nText } from '../../automation/behavior/types.ts';
import { SvgIcon } from './icons/index.ts';

/**
 * Two icon sets the condition editor leans on: one per kind of field, and one
 * per comparison. The comparison glyphs are drawn, not typed, so `>=` and `!==`
 * scale and take the row's color (red while the value is missing).
 */

const FIELD_PATHS: Record<FieldIcon, string> = {
  gift: 'M20 12v10H4V12M2 7h20v5H2zM12 22V7M12 7H7.5a2.5 2.5 0 0 1 0-5C11 2 12 7 12 7zM12 7h4.5a2.5 2.5 0 0 0 0-5C13 2 12 7 12 7z',
  gem: 'M3 9 12 3l9 6-9 12z',
  user: 'M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM4 21v-1a6 6 0 0 1 6-6h4a6 6 0 0 1 6 6v1',
  star: 'm12 3 2.6 5.6 6.4.8-4.7 4.3 1.3 6.3L12 17l-5.6 3 1.3-6.3L3 9.4l6.4-.8z',
  repeat: 'M17 2l4 4-4 4M3 11V9a4 4 0 0 1 4-4h14M7 22l-4-4 4-4M21 13v2a4 4 0 0 1-4 4H3',
  text: 'M4 6h16M4 12h10M4 18h6',
  hash: 'M4 9h16M4 15h16M10 3 8 21M16 3l-2 18',
  clock: 'M12 3a9 9 0 1 1 0 18 9 9 0 0 1 0-18M12 7v5l3 2',
};

const OPERATOR_PATHS: Record<FilterOperator, string[]> = {
  gte: ['M8 4.5 16 10l-8 5.5', 'M6 19.5h12'],
  gt: ['m9 5 7 7-7 7'],
  lte: ['M16 4.5 8 10l8 5.5', 'M6 19.5h12'],
  lt: ['m15 5-7 7 7 7'],
  eq: ['M5 9.5h14M5 14.5h14'],
  neq: ['M5 9.5h14M5 14.5h14', 'M16 4 8 20'],
  in: ['M16 5a8 7 0 1 0 0 14', 'M8 12h9'],
  contains: ['M3.5 6.5h17v11h-17z', 'M9 12h6'],
  'starts-with': ['M3.5 6.5h17v11h-17z', 'M7 12h4'],
  'is-true': ['M12 3a9 9 0 1 1 0 18 9 9 0 0 1 0-18', 'm8 12 3 3 5-6'],
  'is-false': ['M12 3a9 9 0 1 1 0 18 9 9 0 0 1 0-18', 'M9 9l6 6M15 9l-6 6'],
};

/** One wording per operator, used by the table, the summaries and the sentence. */
export const OPERATOR_LABELS: Record<FilterOperator, I18nText> = {
  "gte": { default: "at least", i18key: "behavior.operator.gte" },
  "gt": { default: "more than", i18key: "behavior.operator.gt" },
  "lte": { default: "at most", i18key: "behavior.operator.lte" },
  "lt": { default: "less than", i18key: "behavior.operator.lt" },
  "eq": { default: "is", i18key: "behavior.operator.eq" },
  "neq": { default: "is not", i18key: "behavior.operator.neq" },
  "in": { default: "is one of", i18key: "behavior.operator.in" },
  "contains": { default: "contains", i18key: "behavior.operator.contains" },
  "starts-with": { default: "starts with", i18key: "behavior.operator.starts-with" },
  "is-true": { default: "is true", i18key: "behavior.operator.is-true" },
  "is-false": { default: "is false", i18key: "behavior.operator.is-false" },
};

/** The code equivalent, shown under the name so the symbol is learnable. */
export const OPERATOR_CODE: Record<FilterOperator, string> = {
  gte: '>=',
  gt: '>',
  lte: '<=',
  lt: '<',
  eq: '===',
  neq: '!==',
  in: 'in [ ]',
  contains: 'includes()',
  'starts-with': 'startsWith()',
  'is-true': '=== true',
  'is-false': '=== false',
};

type GlyphProps = { size?: number };

export function FieldIconGlyph({ icon, size = 14 }: GlyphProps & { icon: FieldIcon }) {
  return (
    <SvgIcon size={size} strokeWidth={1.9}>
      <path d={FIELD_PATHS[icon]} />
    </SvgIcon>
  );
}

export function OperatorGlyph({ operator, size = 14 }: GlyphProps & { operator: FilterOperator }) {
  return (
    <SvgIcon size={size} strokeWidth={1.9}>
      {OPERATOR_PATHS[operator].map((path) => <path d={path} key={path} />)}
    </SvgIcon>
  );
}

export default FieldIconGlyph;
</script>
