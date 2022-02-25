import { getRenderer } from 'core/component'
import { Key } from '@raycenity/misc-ts'
import { useEffect, UseEffectRerun, useState } from 'core'

export function useLazy <T> (lazy: T | Promise<T>, loading: T): T {
  if (lazy instanceof Promise) {
    // Try immediate resolve (idk if this ever actually works)
    void lazy.then(resolved => {
      lazy = resolved
    })
  }

  if (lazy instanceof Promise) {
    const [resolved, setResolved] = useState({ value: loading, isLoading: true })
    if (!resolved().isLoading) {
      return resolved().value
    }

    void lazy.then(value => {
      // This will also trigger an update
      setResolved({ value, isLoading: false })
    })

    return loading
  } else {
    // Still need to fill in the state
    void useState({ value: lazy, isLoading: false })
    return lazy
  }
}

export function useInput (handler: (key: Key) => void): void {
  const renderer = getRenderer()
  useEffect(() => {
    return renderer.useInput(handler)
  }, 'on-create')
}

export function useDelay (millis: number, handler: () => void, rerun: UseEffectRerun): void {
  useEffect(() => {
    const timeout = setTimeout(handler, millis)
    return () => clearTimeout(timeout)
  }, rerun)
}

export function useInterval (millis: number, handler: () => void): void {
  useEffect(() => {
    const interval = setInterval(handler, millis)
    return () => clearInterval(interval)
  }, 'on-create')
}
