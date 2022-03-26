// Idk whether to put this in a misc folder, or even misc-ts...

const AUGMENT_SET_RAW: unique symbol = Symbol.for('augment-set-raw')

/** Call `onSet` every time a property might be set, if `value` is an object or function */
export function augmentSetProp<T> (value: T, onSet: (path: string) => void, path: string = ''): T {
  if ((typeof value === 'object' && value !== null) || typeof value === 'function') {
    return augmentSet<T & object>(value as T & (object | Function), onSet, path)
  } else {
    return value
  }
}

/** Call `onSet` every time a property might be set */
export function augmentSet<T extends object | Function> (object: T, onSet: (path: string) => void, path: string = ''): T {
  return new Proxy(object, {
    get: (target: T, p: string | number | symbol): any => {
      if (p === AUGMENT_SET_RAW) {
        return object
      } else {
        const subpath = typeof p === 'string' ? `${path}.${p}` : `${path}[${p.toString()}]`
        return augmentSetProp((target as any)[p], onSet, subpath, target)
      }
    },
    set: (target: T, p: string | number | symbol, value: any): boolean => {
      const subpath = typeof p === 'string' ? `${path}.${p}` : `${path}[${p.toString()}]`;
      (target as any)[p] = value
      onSet(subpath)
      return true
    },
    apply: (target: T, thisArg: any, args: any[]): any => {
      const subpath = `${path}(...)`

      // Answer to https://stackoverflow.com/questions/43236329/why-is-proxy-to-a-map-object-in-es2015-not-working?noredirect=1&lq=1
      const rawThis = thisArg[AUGMENT_SET_RAW] ?? thisArg
      const prototype =
        rawThis === null || rawThis === undefined
          ? null
          : rawThis instanceof Array
            ? Array.prototype
            : rawThis instanceof Map
              ? Map.prototype
              : rawThis instanceof Set
                ? Set.prototype
                : rawThis instanceof WeakMap
                  ? Map.prototype
                  : rawThis instanceof WeakSet
                    ? Set.prototype
                    : Object.getPrototypeOf(rawThis)
      const isIntrinsic = INTRINSIC_PROTOTYPES.has(prototype)

      if (isIntrinsic) {
        const result = Reflect.apply(target as Function, rawThis, args)

        const isPure = INTRINSIC_PROTOTYPES.get(prototype)!.get((target as Function).name)
        switch (isPure) {
          case true:
            break
          case false:
            onSet(subpath)
            break
          default:
            console.warn(`Unknown purity for intrinsic function, please add: ${prototype.toString() as string}.${(target as Function).name}`)
            onSet(subpath)
            break
        }

        return result
      } else {
        return Reflect.apply(target as Function, thisArg, args)
      }
    }
  })
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
