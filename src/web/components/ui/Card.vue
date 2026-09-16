<script lang="tsx">
import type { VNodeChild } from 'vue';
import { defineVueFunctional } from '../../vue/component.ts';
import { InfoTip } from './InfoTip.vue';

type CardProps = {
  children: VNodeChild;
  title?: string;
  subtitle?: string;
  /** Longer explanation behind an info icon next to the title. */
  hint?: string;
  icon?: VNodeChild;
  action?: VNodeChild;
  variant?: 'default' | 'elevated' | 'ghost';
  padding?: 'sm' | 'md' | 'lg' | 'none';
  className?: string;
};

export const Card = defineVueFunctional<CardProps>((props) => {
  const { children, title, subtitle, hint, icon, action, variant = 'default', padding = 'md', className = '' } = props;
  return (
    <div class={`ui-card ui-card--${variant} ui-card--pad-${padding} ${className}`}>
      {title || icon || subtitle || hint || action ? (
        <header class="ui-card__header">
          <div class="ui-card__title-wrap">
            {icon ? <span class="ui-card__icon">{icon}</span> : null}
            <div>
              {title ? (
                <h2 class="ui-card__title">
                  {title}
                  {hint ? <InfoTip text={hint} /> : null}
                </h2>
              ) : null}
              {subtitle ? <p class="ui-card__subtitle">{subtitle}</p> : null}
            </div>
          </div>
          {action ? <div class="ui-card__action">{action}</div> : null}
        </header>
      ) : null}
      <div class="ui-card__body">{children}</div>
    </div>
  );
});

export const CardFooter = defineVueFunctional<{ children?: VNodeChild }>((props) => <div class="ui-card__footer">{props.children}</div>);

export const Badge = defineVueFunctional<{ children?: VNodeChild; tone?: 'neutral' | 'pink' | 'cyan' | 'success' }>((props) => (
  <span class={`ui-badge ui-badge--${props.tone ?? 'neutral'}`}>{props.children}</span>
));

export const Alert = defineVueFunctional<{ children?: VNodeChild; variant?: 'info' | 'success' | 'danger' | 'warning' }>((props) => (
  <div class={`ui-alert ui-alert--${props.variant ?? 'info'}`}>{props.children}</div>
));

export const EmptyState = defineVueFunctional<{ title: string; description?: string; action?: VNodeChild }>((props) => {
  const { title, description, action } = props;
  return (
    <div class="ui-empty">
      <div class="ui-empty__title">{title}</div>
      {description ? <p class="ui-empty__desc">{description}</p> : null}
      {action ? <div class="ui-empty__action">{action}</div> : null}
    </div>
  );
});

export const Chip = defineVueFunctional<{ children?: VNodeChild; onClick?: () => void; active?: boolean }>((props) => {
  const { children, onClick, active } = props;
  return (
    <button type="button" class={`ui-chip ${active ? 'is-active' : ''}`} onClick={onClick}>
      {children}
    </button>
  );
});
export const ChipGroup = defineVueFunctional<{ children?: VNodeChild }>((props) => <div class="ui-chip-group">{props.children}</div>);

export default Card;
</script>
