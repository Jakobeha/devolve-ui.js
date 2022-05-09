import { assert } from '@raycenity/misc-ts'

const LENS_TARGET: unique symbol = Symbol.for('Lens.target')
const LENS_OBSERVERS: unique symbol = Symbol.for('Lens.observers')
const LENS_DEBUG_PATH: unique symbol = Symbol.for('Lens.debugPath')

type Primitive = string | number | symbol | boolean | bigint | null | undefined
/**
 * Functions are technically objects: they can contain properties, the only difference is they are callable and objects aren't.
 * However they aren't `object`s, their typeof is different. It's kind of confusing.
 */
type Object = object | Function

export type Lens<T> =
  (T extends object ? { readonly [K in keyof T]: Lens<T[K]> } : {})
  & (T extends Array<infer E> ? E[] : {})
  & (T extends Set<infer E> ? Set<E> : {})
  & (T extends Map<infer K, infer V> ? Map<K, V> : {}) & {
    readonly [LENS_TARGET]: T
    readonly [LENS_OBSERVERS]: Array<(value: T, debugPath: string) => void>
    readonly [LENS_DEBUG_PATH]: string
    v: T
  }

export function Lens<T> (value: T, debugPath: string = ''): Lens<T> {
  if ((typeof value === 'object' && value !== null) || typeof value === 'function') {
    return lensObject(value as T & Object, debugPath) as unknown as Lens<T>
  } else {
    return lensPrimitive(value as T & Primitive, debugPath) as unknown as Lens<T>
  }
}

export module Lens {
  export function onSet<T> (lens: Lens<T>, onSet: (value: T, debugPath: string) => void): void {
    const observers = lens[LENS_OBSERVERS]
    assert(observers !== undefined, 'not a valid lens')
    const index = observers.indexOf(onSet)
    assert(index === -1, 'setter already added to lens')
    observers.push(onSet)
  }

  export function removeOnSet<T> (lens: Lens<T>, onSet: (value: T, debugPath: string) => void): void {
    const observers = lens[LENS_OBSERVERS]
    assert(observers !== undefined, 'not a valid lens')
    const index = observers.indexOf(onSet)
    assert(index !== -1, 'setter not added to lens')
    observers.splice(index, 1)
  }
}

function lensPrimitive<T extends Primitive> (value: T, debugPath: string): Lens<T> {
  const observers: Array<(value: T, debugPath: string) => void> = []
  return new Proxy({}, {
    get: (_: {}, p: string | number | symbol): any => {
      const subpath = typeof p === 'string' ? `${debugPath}.${p}` : `${debugPath}[${p.toString()}]`
      // 1) Get if value or internal
      switch (p) {
        case LENS_TARGET:
        case 'v':
          return value
        case LENS_OBSERVERS:
          return observers
        case LENS_DEBUG_PATH:
          return debugPath
        case Symbol.toPrimitive:
          // 2) Forgot to call .v
          throw new Error(`forgot to call .v on primitive lens: ${debugPath}. Instead use ${debugPath}.v`)
        default:
          // 3) Not an object, can't get properties
          throw new Error(`this lens is a primitive, it has no properties except v (and internals): ${subpath}`)
      }
    },
    set: (_: {}, p: string | number | symbol, prop: any): boolean => {
      const subpath = typeof p === 'string' ? `${debugPath}.${p}` : `${debugPath}[${p.toString()}]`
      switch (p) {
        case 'v':
          // 1) Set value
          value = prop
          // We use [...observers] because if we add new observers,
          // they should already know the new value, so we don't want to call them as well.
          for (const onSet of [...observers]) {
            onSet(value, debugPath)
          }
          return true
        case LENS_TARGET:
        case LENS_OBSERVERS:
        case LENS_DEBUG_PATH:
          // 2) Can't set internals
          throw new Error(`can't set lens internal property: ${p.toString()}`)
        default:
          // 3) Not an object, can't set properties
          throw new Error(`this lens is a primitive, it has no properties except v (and internals): ${subpath}`)
      }
    }
  }) as unknown as Lens<T>
}

function lensObject<T extends Object> (value: T, debugPath: string): Lens<T> {
  const cache = new Map<string | number | symbol, any>()
  const observers: Array<(value: T, debugPath: string) => void> = []
  return new Proxy({}, {
    get: (_: {}, p: string | number | symbol, receiver?: any): any => {
      const subpath = typeof p === 'string' ? `${debugPath}.${p}` : `${debugPath}[${p.toString()}]`
      // 1) Get if value or internal
      switch (p) {
        case LENS_TARGET:
        case 'v':
          return value
        case LENS_OBSERVERS:
          return observers
        case LENS_DEBUG_PATH:
          return debugPath
        default:
          // 2) Get if cached property
          if (cache.has(p)) {
            return cache.get(p)
          } else {
            // 3) Get if intrinsic function property
            // Answer to https://stackoverflow.com/questions/43236329/why-is-proxy-to-a-map-object-in-es2015-not-working?noredirect=1&lq=1
            const prototype =
              typeof value !== 'object' || value === null
                ? null
                : value instanceof Array
                  ? Array.prototype
                  : value instanceof Map
                    ? Map.prototype
                    : value instanceof Set
                      ? Set.prototype
                      : value instanceof WeakMap
                        ? Map.prototype
                        : value instanceof WeakSet
                          ? Set.prototype
                          : null
            const isIntrinsic = prototype !== null && INTRINSIC_PROTOTYPES.has(prototype)
            if (isIntrinsic) {
              const intrinsicFunction = Reflect.get(value, p, receiver)
              const isPure = typeof p !== 'string' ? undefined : INTRINSIC_PROTOTYPES.get(prototype)!.get(p)
              if (typeof intrinsicFunction === 'function') {
                const subpathApply = `${subpath}(...)`
                return (...args: any[]): any => {
                  const result = intrinsicFunction.apply(value, args)
                  switch (isPure) {
                    case true:
                      break
                    case false:
                      for (const onSet of [...observers]) {
                        onSet(value, subpathApply)
                      }
                      break
                    default:
                      // This lint error is wrong
                      // eslint-disable-next-line @typescript-eslint/no-base-to-string
                      console.warn(`Unknown purity for intrinsic function, please add: ${prototype.toString()}.${p.toString()}`)
                      for (const onSet of [...observers]) {
                        onSet(value, subpathApply)
                      }
                      break
                  }
                  return result
                }
              }
            }

            // 4) Get sublens property
            const initialSubvalue = Reflect.get(value, p, receiver)
            const sublens = Lens(initialSubvalue, subpath)
            Lens.onSet(sublens, newSubvalue => {
              Reflect.set(value, p, newSubvalue)
              for (const onSet of [...observers]) {
                onSet(value, subpath)
              }
            })
            cache.set(p, sublens)
            return sublens
          }
      }
    },
    set: (_: {}, p: string | number | symbol, prop: any): boolean => {
      const subpath = typeof p === 'string' ? `${debugPath}.${p}` : `${debugPath}[${p.toString()}]`
      switch (p) {
        case 'v':
          // 1) Set value
          value = prop
          for (const onSet of [...observers]) {
            onSet(value, debugPath)
          }
          return true
        case LENS_TARGET:
        case LENS_OBSERVERS:
        case LENS_DEBUG_PATH:
          // 2) Can't set internals
          throw new Error(`can't set lens internal property: ${p.toString()}`)
        default:
          // 3) Can't set children directly, use .v = ...
          throw new Error(`can't set directly: ${subpath}. Instead use ${subpath}.v = ...`)
      }
    },
    has (_: {}, p: string | symbol): boolean {
      switch (p) {
        case 'v':
        case LENS_TARGET:
        case LENS_OBSERVERS:
        case LENS_DEBUG_PATH:
          return true
        default:
          return Reflect.has(value, p)
      }
    },
    ownKeys (_: {}): ArrayLike<string | symbol> {
      return [...Reflect.ownKeys(value), 'v', LENS_TARGET, LENS_OBSERVERS, LENS_DEBUG_PATH]
    },
    deleteProperty (_: {}, p: string | symbol): boolean {
      const subpath = typeof p === 'string' ? `${debugPath}.${p}` : `${debugPath}[${p.toString()}]`
      switch (p) {
        case 'v':
          // 1) Can't delete value
          throw new Error(`can't delete lens .v. Delete on the parent: 'delete ${debugPath}`)
        case LENS_TARGET:
        case LENS_OBSERVERS:
        case LENS_DEBUG_PATH:
          // 2) Can't delete internals
          throw new Error(`can't delete lens internal property: ${p.toString()}`)
        default: {
          // 3) Delete child
          cache.delete(p)
          const didDelete = Reflect.deleteProperty(value as any, p)
          if (didDelete) {
            for (const onSet of [...observers]) {
              onSet(value, subpath)
            }
          }
          return didDelete
        }
      }
    }
  }) as unknown as Lens<T>
}

const INTRINSIC_PROTOTYPES: WeakMap<object, Map<string, boolean>> = new WeakMap()

export function registerIntrinsicPrototype (prototype: object | object[], intrinsicFunctions: Array<[string, boolean]>): void {
  // Apparently Array.prototype is an array and iterates as if it were empty
  if (Array.isArray(prototype) && prototype !== Array.prototype) {
    for (const actualPrototype of prototype) {
      registerIntrinsicPrototype(actualPrototype, intrinsicFunctions)
    }
    return
  }
  // Allow already registered prototypes to be extended
  if (!INTRINSIC_PROTOTYPES.has(prototype)) {
    INTRINSIC_PROTOTYPES.set(prototype, new Map())
  }
  const knownPureFunctions = INTRINSIC_PROTOTYPES.get(prototype)!
  for (const [name, isPure] of intrinsicFunctions) {
    knownPureFunctions.set(name, isPure)
  }
}

registerIntrinsicPrototype([Array.prototype], [
  ['map', true],
  ['filter', true],
  ['reduce', true],
  ['reduceRight', true],
  ['forEach', true],
  ['some', true],
  ['every', true],
  ['find', true],
  ['findIndex', true],
  ['copyWithin', true],
  ['flat', true],
  ['flatMap', true],
  ['concat', true],
  ['slice', true],
  ['toSource', true],
  ['push', false],
  ['pop', false],
  ['shift', false],
  ['unshift', false],
  ['splice', false],
  ['reverse', false],
  ['sort', false],
  ['fill', false],
  ['includes', true],
  ['indexOf', true],
  ['lastIndexOf', true],
  ['join', true],
  ['toString', true],
  ['toLocaleString', true],
  ['toJSON', true],
  ['entries', true],
  ['keys', true],
  ['values', true]
])

registerIntrinsicPrototype([Set.prototype], [
  ['has', true],
  ['delete', false],
  ['clear', false],
  ['add', false]
])

registerIntrinsicPrototype(Map.prototype, [
  ['set', false],
  ['get', true],
  ['has', true],
  ['delete', false],
  ['clear', false]
])
