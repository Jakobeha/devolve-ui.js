import { getVComponent, iterVComponentsTopDown } from 'core/component'
import { rec } from '@raycenity/misc-ts'

/**
 * Allows you to pass props implicitly, similar to React contexts.
 * However contexts in devolve-ui work slightly different: they are hooks instead of components.
 *
 * Usage:
 * ```jsx
 * const fooBarContext = createContext<FooBar>()
 *
 * const Parent = () => {
 *   fooBarContext.useProvide({ foo: 'bar' })
 *   return <box><Child /></box>
 * }
 *
 * const Child = () => {
 *   const value = fooBarContext.useConsume()
 *   // value is { foo: 'bar' }
 *   return <text>{value.foo}</text>
 * }
 * ```
 */
export interface Context<T> {
  /** In child components, `useConsume` will return the input value */
  useProvide: (value: T) => void
  /** Returns the value passed to `useProvide` in the nearest parent component,
   * or the context's default value if no `useProvide`,
   * or throws an error if there is no default value
   */
  useConsume: () => T
}

/**
 * Creates a context.
 *
 * If `defaultValue` is provided (*not undefined*),
 * it will be returned if any child calls `useConsume` without a parent having called `useProvide`.
 * Otherwise, `useConsume` without a provided value will throw an error.
 */
export function createContext<T> (initialValue?: T): Context<T> {
  return rec<Context<T>>(context => ({
    useProvide: (value: T): void => {
      const component = getVComponent()
      if (component.contexts.has(context)) {
        throw new Error('This context was already provided in this component')
      }
      component.contexts.set(context, value)
    },
    useConsume: (): T => {
      for (const component of iterVComponentsTopDown()) {
        if (component.contexts.has(context)) {
          return component.contexts.get(context)
        }
      }
      if (initialValue !== undefined) {
        return initialValue
      }
      throw new Error('This context was not provided in any parent component, and has no initial value')
    }
  }))
}
