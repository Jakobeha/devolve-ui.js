import { getVComponent, VComponent } from 'core/component'
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
  /** Returns the value passed to `useProvide` in the nearest parent component.
   * This is guaranteed to return `null` the first time the component is created,
   * because child components are created before their parents.
   * If the child changes parents, it will be updated and useConsume will return the new parent's context.
   * or the context's default value if no `useProvide`,
   * or throws an error if there is no default value
   */
  useConsume: () => T | null
}

let CONTEXT_DEBUG_ID = 0

/**
 * Creates a context.
 */
export function createContext<T> (): Context<T> {
  const contextId = CONTEXT_DEBUG_ID++
  return rec<Context<T>>(context => ({
    useProvide: (value: T): void => {
      const component = getVComponent()
      if (component.providedContexts.has(context)) {
        throw new Error('This context was already provided in this component')
      }
      VComponent.setProvidedContext(component, context, value, false, `context-${contextId}`)
    },
    useConsume: (): T | null => {
      const component = getVComponent()
      if (component.consumedContexts.has(context)) {
        return component.consumedContexts.get(context)
      }
      component.consumedContexts.set(context, null)
      return null
    }
  }))
}
