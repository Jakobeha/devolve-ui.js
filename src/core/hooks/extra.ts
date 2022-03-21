import { getRenderer } from 'core/component'
import { Key } from '@raycenity/misc-ts'
import { useDynamic, useEffect, UseEffectRerun, useStateFast } from 'core'

/** Returns a function which will always be called with the latest props and state dependencies. */
export function useDynamicFn<Parameters extends any[], Return> (
  fn: (...args: Parameters) => Return
): (...args: Parameters) => Return {
  const getFn = useDynamic(fn)
  return (...args: Parameters): Return => getFn()(...args)
}

/**
 * Lazily computes a value and then updates with the returned value.
 * Subsequent calls use the returned value, so it's not recalculated.
 */
export function useLazy <T> (lazy: T | Promise<T>, loading: T): T {
  if (lazy instanceof Promise) {
    // Try immediate resolve (idk if this ever actually works)
    void lazy.then(resolved => {
      lazy = resolved
    })
  }

  if (lazy instanceof Promise) {
    const [resolved, setResolved] = useStateFast({ value: loading, isLoading: true })
    if (!resolved.isLoading) {
      return resolved.value
    }

    void lazy.then(value => {
      // This will also trigger an update
      setResolved({ value, isLoading: false })
    })

    return loading
  } else {
    // Still need to fill in the state
    void useStateFast({ value: lazy, isLoading: false })
    return lazy
  }
}

/**
 * Read keyboard input inside of your component.
 */
export function useInput (handler: (key: Key) => void): void {
  handler = useDynamicFn(handler)

  const renderer = getRenderer()
  useEffect(() => {
    return renderer.useInput(handler)
  }, 'on-create')
}

/**
 * Performs an action after the specified delay.
 *
 * Then performs the action again after the specified delay every time `rerun` is triggered (@see `useEffect` for how `rerun` works).
 */
export function useDelay (millis: number, handler: () => void, rerun: UseEffectRerun): void {
  handler = useDynamicFn(handler)

  useEffect(() => {
    const timeout = setTimeout(handler, millis)
    return () => clearTimeout(timeout)
  }, rerun)
}

/**
 * Performs an action every `millis` milliseconds while the component is alive.
 */
export function useInterval (millis: number, handler: () => void): void {
  handler = useDynamicFn(handler)

  useEffect(() => {
    const interval = setInterval(handler, millis)
    return () => clearInterval(interval)
  }, 'on-create')
}
