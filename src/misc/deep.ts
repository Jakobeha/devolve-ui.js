import { BackRef } from 'misc/ref'

export type DeepPartial<T> = {
  [K in keyof T]?: T extends {} ? DeepPartial<T[K]> : T[K]
}

/** create an object with circular referneces */
export function rec <T extends object> (build: (value: T) => T): T {
  const value: Partial<T> = {}
  Object.assign(value, build(value as T))
  return value as T
}

/** create an object with circular references in 2 stages */
export function rec2 <T extends object, K1 extends keyof T> (
  build1: (value: T) => Omit<T, K1>,
  build2: (value: T) => Pick<T, K1>
): T {
  const value: Partial<T> = {}
  Object.assign(value, build1(value as T))
  Object.assign(value, build2(value as T))
  return value as T
}

/** create an object with circular references in 3 stages */
export function rec3 <T1 extends object, T2 extends object, T3 extends object> (
  build1: (value: object) => T1,
  build2: (value: T1) => T2,
  build3: (value: T1 & T2) => T3
): T1 & T2 & T3 {
  const value: Partial<T1 & T2 & T3> = {}
  Object.assign(value, build1(value))
  Object.assign(value, build2(value as T1))
  Object.assign(value, build3(value as T1 & T2))
  return value as T1 & T2 & T3
}

export function equals (a: any, b: any): boolean {
  if (typeof a === 'object' && typeof b === 'object') {
    return JSON.stringify(a) === JSON.stringify(b)
  } else {
    return a === b
  }
}

export function deepMerge<A, B> (a: A, b: B, debugPath = ''): A & B {
  if ((a as any) === (b as any)) {
    return a as A & B
  } else if (typeof a === 'object' && typeof b === 'object') {
    const result: Partial<A & B> = Object.assign({}, a)
    for (const key in b) {
      if (key in result) {
        // @ts-expect-error
        result[key] = deepMerge(result[key], b[key], `${debugPath}.${key}`)
      } else {
        // @ts-expect-error
        result[key] = b[key]
      }
    }
    return result as A & B
  } else if (typeof a === 'function' && typeof b === 'function') {
    return ((...args: any[]) => {
      return deepMerge(a(...args), b(...args), `${debugPath}(...)`)
    }) as unknown as A & B
  } else if (typeof a === 'function' && typeof b === 'object') {
    return ((...args: any[]) => {
      return deepMerge(a(...args), b, `${debugPath}(...)`)
    }) as unknown as A & B
  } else if (typeof a === 'object' && typeof b === 'function') {
    return ((...args: any[]) => {
      return deepMerge(a, b(...args), `${debugPath}(...)`)
    }) as unknown as A & B
  } else if (a === undefined || a === null) {
    return b as A & B
  } else if (b === undefined || b === null) {
    return a as A & B
  } else {
    throw new Error(
      `Cannot merge different non-objects/functions/nulls: error at path '${debugPath}'`
    )
  }
}

export function deepCopy <T> (value: T, copyMap: Map<any, any> = new Map<any, any>(), debugPath = ''): T {
  if (typeof value === 'object') {
    if (value instanceof BackRef) {
      const refCopy = copyMap.get(value.get())
      if (refCopy !== undefined) {
        const copy = new BackRef(refCopy, value.id) as any
        copyMap.set(value, copy)
        return copy
      } else {
        // References something outside the copy
        return value
      }
    } else if (/* assumes frozen values are deep-frozen */ Object.isFrozen(value)) {
      return value
    } else if (Array.isArray(value)) {
      const copy = value.map((elem, i) => deepCopy(elem, copyMap, `${debugPath}[${i}]`)) as any
      copyMap.set(value, copy)
      return copy
    } else {
      const copy: Partial<T> = {}
      for (const key in value) {
        copy[key] = deepCopy(value[key], copyMap, `${debugPath}.${key}`)
      }
      copyMap.set(value, copy)
      return copy as T
    }
  } else {
    return value
  }
}
