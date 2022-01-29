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

export function useEffect (effect: () => void | Promise<void>): void {
  const component = getVComponent()
  // eslint-disable-next-line @typescript-eslint/strict-boolean-expressions
  if (component.isBeingConstructed) {
    component.onChange.push(effect)
  }
}
