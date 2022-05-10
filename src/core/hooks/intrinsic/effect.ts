import { getVComponent } from 'core/component'
import { _useDynamicState } from 'core/hooks/intrinsic/state-dynamic'

export interface UseEffectRerunOnChange<Dep> { onChange: Dep[], compare?: (lhs: Dep, rhs: Dep) => boolean }
export interface UseEffectRerunOnDefine<Dep> { onDefine: Array<Dep | undefined> }

export type UseEffectRerun =
  'on-update' |
  'on-create' |
  UseEffectRerunOnChange<any> |
  UseEffectRerunOnDefine<any>

/**
 * Returns an effect which will be called according to `rerun`:
 *
 * - `on-update`: Called every time the component updates. This means it will work the same as putting the code outside of `useEffect`, except effects are delayed until after the component is created.
 * - `on-create`: Called only once when the component is created. Not called in subsequent updates.
 * - `{ onChange: deps, compare? }`: Called when `deps` change, compares each dependency using `compare` if provided (otherwise `===`).
 * - `{ onDefine: deps }`: Called when `deps` change and every one of `deps` becomes undefined
 * - `{ onTrue: () => boolean }`: Called when the return value of `() => boolean` is true (TODO not implemented).
 */
// eslint-disable-next-line @typescript-eslint/no-invalid-void-type
export function useEffect (effect: () => void | (() => void), rerun: UseEffectRerun): void {
  const component = getVComponent()
  if (rerun === 'on-update') {
    component.effects.push(() => {
      const destructor = effect()
      if (typeof destructor === 'function') {
        component.updateDestructors.push(destructor)
        // Update destructors aren't run on permanent destruct
        component.permanentDestructors.push(destructor)
      }
    })
  } else if (rerun === 'on-create') {
    if (component.isBeingCreated) {
      component.effects.push(() => {
        const destructor = effect()
        if (typeof destructor === 'function') {
          component.permanentDestructors.push(destructor)
        }
      })
    }
  } else if ('onChange' in rerun) {
    const ourMemo = rerun.onChange
    const compare = rerun.compare ?? ((lhs: any, rhs: any) => lhs === rhs)
    const [getMemo, setMemo] = _useDynamicState(ourMemo, false)
    const [getDestructor, setDestructor] = _useDynamicState<(() => void) | null>(null, false)
    const memo = getMemo()
    component.effects.push(() => {
      let doEffect = false
      if (component.isBeingCreated) {
        doEffect = true
      } else {
        if (memo.length !== ourMemo.length) {
          throw new Error(`number of dependencies changed in between component update (you can't do that): ${memo.length} to ${ourMemo.length}`)
        }
        for (let i = 0; i < memo.length; i++) {
          if (!compare(memo[i], ourMemo[i])) {
            doEffect = true
            break
          }
        }
      }

      if (doEffect) {
        const oldDestructor = getDestructor()
        if (oldDestructor !== null) {
          component.permanentDestructors.splice(component.permanentDestructors.indexOf(oldDestructor), 1)
        }
        const destructor = effect()
        if (typeof destructor === 'function') {
          setDestructor(destructor)
          component.permanentDestructors.push(destructor)
        }
      }
    })
    if (!component.isBeingCreated) {
      setMemo(ourMemo)
    }
  } else if ('onDefine' in rerun) {
    const deps = rerun.onDefine
    const depsWereDefined = !deps.some(dep => dep === undefined)
    const [lastDepsWereDefined, setLastDepsWereDefined] = _useDynamicState(false, false)
    if (depsWereDefined && !lastDepsWereDefined()) {
      component.effects.push(() => {
        const destructor = effect()
        if (typeof destructor === 'function') {
          const updateDestructor = (): void => {
            if (!lastDepsWereDefined()) {
              destructor()
            } else {
              component.nextUpdateDestructors.push(updateDestructor)
            }
          }
          component.updateDestructors.push(updateDestructor)
          // Update destructors aren't run on permanent destruct (we take advantage of that here w/ different update destructor)
          component.permanentDestructors.push(destructor)
        }
      })
    }
    setLastDepsWereDefined(depsWereDefined)
  }
}
