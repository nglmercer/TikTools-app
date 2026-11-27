/** Hands a fake host's reply to a generic caller (`call<T>`): the store under test validates the shape, not the type system. */
const identity: Function = (value: unknown) => value;

export function answer<T>(value: unknown): T {
  // The untyped Reflect.apply overload returns any, so the reply flows into the caller's chosen T without a cast.
  return Reflect.apply(identity, undefined, [value]);
}
