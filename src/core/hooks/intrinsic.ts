import { getVComponent, VComponent } from 'core/component'

/**
 * Returns a value and setter.
 *
 * If you call the setter, when the component updates, it will return the set value instead of `initialValue`.
 */
export function useState<T> (initialState: T): [T, (newState: T) => void] {
  const [get, set] = _useDynamicState(initialState, true)
  return [get(), set]
}

/**
 * Returns a function which will update with the last value passed into it,
 * for use in asynchronous effects.
 *
 * Check out `useDynamicFn` as it can often allow you to avoid this,
 * by making the calling function itself update every time the component updates.
 */
export function useDynamic<T> (value: T): () => T {
  const [get, set] = _useDynamicState(value, false)
  set(value)
  return get
}

export function _useDynamicState<T> (initialState: T, doUpdate: boolean): [() => T, (newState: T) => void] {
  const component = getVComponent()
  const index = component.nextStateIndex++
  if (component.isBeingCreated) {
    if (component.state.length !== index) {
      throw new Error(`sanity check failed: state length (${component.state.length}) !== index (${index})`)
    }
    component.state.push(initialState)
  }

  return [
    () => component.state[index],
    (newState: T) => {
      component.state[index] = newState
      if (doUpdate) {
        VComponent.update(component)
      }
    }
  ]
}

interface UseEffectRerunOnChange<Dep> { onChange: Dep[], compare?: (lhs: Dep, rhs: Dep) => boolean }
interface UseEffectRerunOnTrue { onTrue: () => boolean }

export type UseEffectRerun =
  'on-update' |
  'on-create' |
  UseEffectRerunOnChange<any> |
  UseEffectRerunOnTrue

/**
 * Returns an effect which will be called according to `rerun`:
 *
 * - `on-update`: Called every time the component updates. This means it will work the same as putting the code outside of `useEffect`, except effects are delayed until after the component is created.
 * - `on-create`: Called only once when the component is created. Not called in subsequent updates.
 * - `{ onChange: deps, compare? }`: Called when `deps` change, compares each dependency using `compare` if provided (otherwise `===`).
 * - `{ onTrue: () => boolean }`: Called when the return value of `() => boolean` is true (TODO not implemented).
 */
// eslint-disable-next-line @typescript-eslint/no-invalid-void-type
export function useEffect (effect: () => void | (() => void), rerun: UseEffectRerun): void {
  const component = getVComponent()
  if (rerun === 'on-update') {
    component.effects.push(() => {
      const result = effect()
      if (typeof result === 'function') {
        component.updateDestructors.push(result)
        // Update destructors aren't run on permanent destruct
        component.permanentDestructors.push(result)
      }
    })
  } else if (rerun === 'on-create') {
    if (component.isBeingCreated) {
      component.effects.push(() => {
        const result = effect()
        if (typeof result === 'function') {
          component.permanentDestructors.push(result)
        }
      })
    }
  } else if ('onChange' in rerun) {
    const ourMemo = rerun.onChange
    const compare = rerun.compare ?? ((lhs: any, rhs: any) => lhs === rhs)
    const [memo, setMemo] = _useDynamicState(ourMemo, false)
    component.effects.push(() => {
      let doEffect = false
      if (component.isBeingCreated) {
        doEffect = true
      } else {
        if (memo.length !== ourMemo.length) {
          throw new Error('number of dependencies changed in between component update (you can\'t do that)')
        }
        for (let i = 0; i < memo.length; i++) {
          if (!compare(memo()[i], ourMemo[i])) {
            doEffect = true
            break
          }
        }
      }

      if (doEffect) {
        const result = effect()
        if (typeof result === 'function') {
          throw new Error('you can\'t have a destructor in an update with dependencies, because we don\'t know when the dependencies will change! Put your destructor code directly in useEffect and track the dependency change for something similar')
        }
      }
    })
    if (!component.isBeingCreated) {
      setMemo(ourMemo)
    }
  } else if ('onTrue' in rerun) {
    throw new Error('TODO not implemented')
  }
}
