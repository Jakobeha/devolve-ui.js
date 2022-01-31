import { getVComponent, getVRenderer, VComponent } from 'core/component'

export function useState<T> (initialState: T): [T, (newState: T) => void] {
  return _useState(initialState, true)
}

function _useState<T> (initialState: T, doUpdate: boolean): [T, (newState: T) => void] {
  const component = getVComponent()
  const index = component.nextStateIndex++
  let state: T
  // Um this is a boolean
  // eslint-disable-next-line @typescript-eslint/strict-boolean-expressions
  if (component.isBeingConstructed) {
    if (component.state.length !== index) {
      throw new Error(`sanity check failed: state length (${component.state.length}) !== index (${index})`)
    }
    component.state.push(initialState)
    state = initialState
  } else {
    state = component.state[index]
  }

  return [
    state,
    (newState: T) => {
      component.state[index] = newState
      if (doUpdate) {
        VComponent.update(component)
      }
    }
  ]
}

// eslint-disable-next-line @typescript-eslint/no-invalid-void-type
export function useEffect (effect: () => void | (() => void) | Promise<void>, deps: any[], compareDeps?: (lhs: any, rhs: any) => boolean): void {
  const component = getVComponent()
  if (deps.length > 0) {
    const [memo, setMemo] = _useState(deps, false)
    // eslint-disable-next-line @typescript-eslint/strict-boolean-expressions
    if (component.isBeingConstructed) {
      compareDeps = compareDeps ?? ((lhs, rhs) => lhs === rhs)

      let memoNow = [...memo]
      component.onChange.push(() => {
        if (memo.length !== memoNow.length) {
          throw new Error('sanity check failed, you can\'t change the number of dependencies')
        }
        let doEffect = false
        for (let i = 0; i < memo.length; i++) {
          if (!compareDeps!(memo[i], memoNow[i])) {
            doEffect = true
            break
          }
        }
        if (doEffect) {
          memoNow = [...memo]
          const result = effect()
          if (typeof result === 'function') {
            component.onDestroy.push(result)
          }
        }
      })
    } else {
      setMemo(deps)
    }
  } else {
    // eslint-disable-next-line @typescript-eslint/strict-boolean-expressions
    if (component.isBeingConstructed) {
      const index = component.onChange.push(() => {
        component.onChange[index] = null
        const result = effect()
        if (typeof result === 'function') {
          component.onDestroy.push(result)
        }
      })
    }
  }
}

export function useInput (handler: (input: string, key: KeyboardEvent) => void): void {
  const renderer = getVRenderer()
  useEffect(() => renderer.useInput(handler), [])
}
