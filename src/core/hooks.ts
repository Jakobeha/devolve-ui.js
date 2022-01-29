import { getVComponent, VComponent } from 'core/vdom'

export function useState<T>(initialState: T): [T, (newState: T) => void] {
  const component = getVComponent()
  let index = component.nextStateIndex++
  let state: T
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

export function useEffect(effect: () => void | Promise<void>) {
  const component = getVComponent()
  if (component.isBeingConstructed) {
    component.onChange.push(effect)
  }
}
