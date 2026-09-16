import { defineComponent, type DefineSetupFnComponent, type SetupContext, type VNodeChild } from 'vue';

export type VueProps = object;

type PublicVueProps<Props extends VueProps> = Omit<Props, 'children'> & {
  children?: Props extends { children?: infer Child } ? Child : VNodeChild;
};

/**
 * Keeps the shared Vue render-function surface small while giving every
 * component a real Vue setup boundary. Props are declared at runtime so updates remain
 * reactive, and the default slot is exposed as the familiar `children` value
 * used by the UI primitives.
 */
export function defineVueComponent<Props extends VueProps>(
  propNames: readonly (keyof Props & string)[],
  setup: (props: Props, context: SetupContext) => () => VNodeChild,
): DefineSetupFnComponent<PublicVueProps<Props>> {
  return defineComponent<Props>((rawProps, context) => {
      const propsWithSlots = new Proxy(rawProps as Props, {
        get(target, key, receiver) {
          // Declared-but-absent props exist on the resolved props object as
          // `undefined`, so presence alone must not shadow the slot/attr
          // fallback: an explicit value wins, otherwise the default slot (for
          // children) or the fallthrough class (for className) applies.
          if (key === 'children') {
            const explicit = Reflect.get(target, key, receiver);
            return explicit !== undefined ? explicit : context.slots.default?.();
          }
          if (key === 'className') {
            const explicit = Reflect.get(target, key, receiver);
            return explicit !== undefined ? explicit : context.attrs.class;
          }
          if (Reflect.has(target, key)) return Reflect.get(target, key, receiver);
          return context.attrs[key as string];
        },
      });
      return setup(propsWithSlots, context);
    }, {
    inheritAttrs: false,
    props: propNames as (keyof Props)[],
  }) as unknown as DefineSetupFnComponent<PublicVueProps<Props>>;
}

/**
 * Stateless components can render directly from the current attrs object.
 * This also normalizes Vue's slot-based children into the old `children` prop.
 */
export function defineVueFunctional<Props extends VueProps>(
  render: (props: Props, context: SetupContext) => VNodeChild,
): DefineSetupFnComponent<PublicVueProps<Props>> {
  return defineComponent<Props>((_props, context) => {
      return () => {
        const props = {
          ...context.attrs,
          ...(context.slots.default ? { children: context.slots.default() } : {}),
        } as Props;
        return render(props, context);
      };
    }, {
    inheritAttrs: false,
    props: [],
  }) as unknown as DefineSetupFnComponent<PublicVueProps<Props>>;
}

export type VueEvent<T extends EventTarget> = Event & { currentTarget: T };
export type VueKeyboardEvent<T extends EventTarget> = KeyboardEvent & { currentTarget: T };
export type VueSubmitEvent<T extends HTMLFormElement = HTMLFormElement> = SubmitEvent & {
  currentTarget: T;
};
