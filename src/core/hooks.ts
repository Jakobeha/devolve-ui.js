import { getVComponent, VComponent } from 'core/component'

export function useState<T> (initialState: T): [T, (newState: T) => void] {
  const component = getVComponent()
  const index = component.nextStateIndex++
  let state: T
  // Um this is a boolean
  // eslint-disable-next-line @typescript-eslint/strict-boolean-expressions
  if (component.isBeingConstructed) {
    if (component.state.length !== index) {
      throw new Error('sanity check failed')
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
      VComponent.update(component)
    }
  ]
}

export function useEffect (effect: () => void | Promise<void>, deps?: any[], compareDeps?: (lhs: any, rhs: any) => boolean): void {
  const component = getVComponent()
  if (deps !== undefined) {
    const [memo, setMemo] = useState(deps)
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
          void effect()
        }
      })
    } else {
      setMemo(deps)
    }
  } else {
    // eslint-disable-next-line @typescript-eslint/strict-boolean-expressions
    if (component.isBeingConstructed) {
      component.onChange.push(effect)
    }
  }
}
