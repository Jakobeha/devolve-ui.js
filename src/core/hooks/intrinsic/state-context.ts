import { augmentSet } from 'core/augment-set'
import { getRenderer, getVComponent, isDebugMode, iterVComponentsTopDown, VComponent } from 'core/component'
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import type { Context } from 'core/hooks/intrinsic/context'
import { useState, useStateFast } from 'core/hooks/intrinsic/state-dynamic'
import { RendererImpl } from 'renderer/common'
import { rec } from '@raycenity/misc-ts'

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
export type StateContext<T> = _StateContext<T, { v: T }>

/**
 * {@link StateContext} but with `useStateFast` instead of `useState`,
 * so improved performance (no proxy) but additional rules to avoid stale values.
 *
 * @see {@link StateContext}
 */
export type FastStateContext<T> = _StateContext<T, [T, (value: T) => void]>

interface _StateContext<T, State> {
  /** Like `useState` but in child components, `useConsume` will return the same state
   * (as in, when the child modifies the state it also modifies the parent state) */
  useProvide: (value: T) => State
  /** Returns the state from `useProvide` in the nearest parent component,
   * or the context's root state if no `useProvide`,
   * or throws an error if there is no root state
   */
  useConsume: () => State
}

let STATE_CONTEXT_DEBUG_ID = 0

/**
 * Creates a state context.
 *
 * If `defaultValue` is provided (*not undefined*), that will be the root state (root is per renderer).
 * Otherwise, `useConsume` without a provided state will throw an error.
 */
export function createStateContext<T> (initialValue?: T): StateContext<T> {
  return _createStateContext(
    initialValue,
    useState,
    (renderer: RendererImpl<any, any>): { v: T } => augmentSet({ v: initialValue! }, path => {
      const stackTrace = isDebugMode()
        ? (new Error().stack?.replace('\n', '  \n') ?? 'could not get stack, new Error().stack is undefined')
        : 'omitted in production'
      VComponent.update(renderer.rootComponent!, `set-context-root-state-${STATE_CONTEXT_DEBUG_ID}-${path}\n${stackTrace}`)
    })
  )
}

/**
 * Creates a fast state context.
 *
 * If `defaultValue` is provided (*not undefined*), that will be the root state (root is per renderer).
 * Otherwise, `useConsume` without a provided state will throw an error.
 */
export function createFastStateContext<T> (initialValue?: T): FastStateContext<T> {
  return _createStateContext(
    initialValue,
    useStateFast,
    (renderer: RendererImpl<any, any>): [T, (value: T) => void] => {
      const rootState: [T, (value: T) => void] = [
        initialValue!,
        value => {
          rootState[0] = value
          const stackTrace = isDebugMode()
            ? (new Error().stack?.replace('\n', '  \n') ?? 'could not get stack, new Error().stack is undefined')
            : 'omitted in production'
          VComponent.update(renderer.rootComponent!, `set-context-root-state-${STATE_CONTEXT_DEBUG_ID}\n${stackTrace}`)
        }
      ]
      return rootState
    }
  )
}

function _createStateContext<T, State> (
  initialValue: T | undefined,
  useState: (initialValue: T) => State,
  mkRootState: (renderer: RendererImpl<any, any>) => State
): _StateContext<T, State> {
  const rootStates: Map<RendererImpl<any, any>, State> | undefined = initialValue === undefined ? undefined : new Map()

  STATE_CONTEXT_DEBUG_ID++

  return rec<_StateContext<T, State>>(context => ({
    useProvide: (value: T): State => {
      const component = getVComponent()
      if (component.contexts.has(context)) {
        throw new Error('This context was already provided in this component')
      }
      const state = useState(value)
      component.contexts.set(context, state)
      return state
    },
    useConsume: (): State => {
      for (const component of iterVComponentsTopDown()) {
        if (component.contexts.has(context)) {
          return component.contexts.get(context)
        }
      }
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
      throw new Error('This context was not provided in any parent component, and has no root state')
    }
  }))
}
