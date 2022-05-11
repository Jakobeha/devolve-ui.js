import { Context } from 'core/hooks/intrinsic/context'
import { getRenderer, getVComponent, iterVComponentsStackTopDown, VComponent } from 'core/component'
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import type { PropsContext } from 'core/hooks/intrinsic/props-context'
import { useState } from 'core/hooks/intrinsic/state-dynamic'
import { RendererImpl } from 'renderer/common'
import { rec } from '@raycenity/misc-ts'
import { Lens } from 'core/lens'

/**
 * Like {@link PropsContext}, allows you to implicitly pass a value from parent to children.
 * However, now the context is also a state and can be mutated by children.
 *
 * Usage:
 * ```jsx
 * const fooBarContext = createStateContext<FooBar>()
 *
 * const Parent = () => {
 *   const [value, setValue] = fooBarContext.useProvide({ foo: 'bar' })
 *   // value is { foo: 'bar' } for the first 5 seconds, and { foo: 'baz' } after
 *   // because Child sets it
 *   return <box><MutatingChild /></box>
 * }
 *
 * const MutatingChild = () => {
 *   const [value, setValue] = fooBarContext.useConsume()
 *   // value is { foo: 'bar' } for the first 5 seconds, and { foo: 'baz' } after
 *   useDelay(5000, () => { setValue({ foo: 'baz' }) })
 *   return <text>{value.foo}</text>
 * }
 * ```
 *
 * @see {@link Context}
 */
export interface StateContext<T> extends Context {
  /** children `useConsume` will point to this state context.
   * Kind of like `useState(initialValue)` but accessible to children. */
  useProvide: (initialValue: T) => Lens<T>
  /**
   * Returns the state from `useProvide` in the nearest parent component.
   * This is guaranteed to be `null` the first time the component is created,
   * because it's created before its parent.
   * If the child changes parents, it will be updated and useConsume will return the new parent's context.
   */
  useConsume: () => Lens<T> | null
  /**
   * If the component was created with a default initial value, returns the context's root state (dependent on renderer).
   * Otherwise there is no root state and this throws an error
   */
  useConsumeRoot: () => Lens<T>
}

let STATE_CONTEXT_DEBUG_ID = 0

/**
 * Creates a state context.
 *
 * If `defaultInitialValue` is provided (*not undefined*), that will be the root initial value (root is per renderer).
 * Otherwise, `useConsume` without a provided state will throw an error.
 */
export function createStateContext<T> (defaultInitialValue?: T): StateContext<T> {
  const contextId = STATE_CONTEXT_DEBUG_ID++
  const mkRootState = (): Lens<T> => Lens<T>(defaultInitialValue!)
  const rootStates: Map<RendererImpl<any, any>, Lens<T>> | undefined = defaultInitialValue === undefined ? undefined : new Map()

  return rec<StateContext<T>>(context => ({
    useProvide: (value: T): Lens<T> => {
      const component = getVComponent()
      if (component.providedContexts.has(context)) {
        throw new Error('This context was already provided in this component')
      }
      // Don't need to explicitly track state since it's ours, so state is tracked implicitly
      const state = useState(value)
      VComponent.setProvidedContext(component, context, state)
      return state
    },
    useConsume: (): Lens<T> | null => {
      // Use assigned
      const component = getVComponent()
      if (component.consumedContexts.has(context)) {
        return component.consumedContexts.get(context)
      }
      // Try to find in hierarchy
      for (const parent of iterVComponentsStackTopDown()) {
        if (parent.providedContexts.has(context)) {
          const state = parent.providedContexts.get(context)
          component.consumedContexts.set(context, state)
          return state
        }
      }
      // Not found
      component.consumedContexts.set(context, null)
      return null
    },
    useConsumeRoot: (): Lens<T> => {
      // Use assigned
      const component = getVComponent()
      if (component.consumedContexts.has(context)) {
        return component.consumedContexts.get(context)
      }
      // Get root state
      if (rootStates !== undefined) {
        const renderer = getRenderer()
        let rootState: Lens<T>
        if (rootStates.has(renderer)) {
          rootState = rootStates.get(renderer)!
        } else {
          // First time anyone accessed root so we need to create it
          rootState = mkRootState()
          rootStates.set(renderer, rootState)
        }
        component.consumedContexts.set(context, rootState)
        // Need to explicitly track state since it isn't ours
        // (even without indirect children, since the state doesn't belong to our parent,
        //  it doesn't belong to any component)
        VComponent.trackState(component, rootState, `consumed-context-changed-${contextId}`)
        return rootState
      }
      // There is no root state
      throw new Error('This context was not created with a defaultInitialValue, so it has no root state')
    },
    isStateContext: true,
    debugId: `state-#${contextId}`
  }))
}
