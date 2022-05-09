import { getRenderer, getVComponent, isDebugMode, VComponent } from 'core/component'
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import type { Context } from 'core/hooks/intrinsic/context'
import { useState } from 'core/hooks/intrinsic/state-dynamic'
import { RendererImpl } from 'renderer/common'
import { rec } from '@raycenity/misc-ts'
import { Lens } from 'core/lens'

/**
 * Like {@link Context}, allows you to implicitly pass a value from parent to children.
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
export interface StateContext<T> {
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
  const mkRootState = (renderer: RendererImpl<any, any>): Lens<T> => {
    const rootState = Lens<T>(defaultInitialValue!)
    Lens.onSet(rootState, (newValue, debugPath) => {
      const stackTrace = isDebugMode()
        ? (new Error().stack?.replace('\n', '  \n') ?? 'could not get stack, new Error().stack is undefined')
        : 'omitted in production'
      VComponent.update(renderer.rootComponent!, `set-context-root-state-${contextId}-${debugPath}\n${stackTrace}`)
    })
    return rootState
  }
  const rootStates: Map<RendererImpl<any, any>, Lens<T>> | undefined = defaultInitialValue === undefined ? undefined : new Map()

  return rec<StateContext<T>>(context => ({
    useProvide: (value: T): Lens<T> => {
      const component = getVComponent()
      if (component.providedContexts.has(context)) {
        throw new Error('This context was already provided in this component')
      }
      const state = useState(value)
      VComponent.setProvidedContext(component, context, state, true, `state-context-${contextId}`)
      return state
    },
    useConsume: (): Lens<T> | null => {
      const component = getVComponent()
      if (component.consumedContexts.has(context)) {
        return component.consumedContexts.get(context)
      }
      /* // Current component needs explicit trackState since it didn't create the state
      VComponent.trackState(component, state, `consumed-context-changed-${contextId}`) */
      component.consumedContexts.set(context, null)
      return null
    },
    useConsumeRoot: (): Lens<T> => {
      if (rootStates !== undefined) {
        const renderer = getRenderer()
        if (rootStates.has(renderer)) {
          return rootStates.get(renderer)!
        } else {
          const rootState = mkRootState(renderer)
          rootStates.set(renderer, rootState)
          return rootState
        }
      }
      throw new Error('This context was not created with a defaultInitialValue, so it has no root state')
    }
  }))
}
