// Idk whether to put this in a misc folder, or even misc-ts...

/** Call `onSet` every time a property might be set, if `value` is an object or function */
export function augmentSetProp<T> (value: T, onSet: (path: string) => void, path: string = '', target: object | null = null): T {
  if (typeof value === 'object') {
    return augmentSet<T & object>(value as T & object, onSet, path)
  } else if (typeof value === 'function') {
    // Answer to https://stackoverflow.com/questions/43236329/why-is-proxy-to-a-map-object-in-es2015-not-working?noredirect=1&lq=1
    return augmentSet<T & Function>(value.bind(target), onSet, path)
  } else {
    return value
  }
}

/** Call `onSet` every time a property might be set */
export function augmentSet<T extends object | Function> (object: T, onSet: (path: string) => void, path: string = ''): T {
  return new Proxy(object, {
    get: (target: T, p: string | number | symbol): any => {
      const subpath = typeof p === 'string' ? `${path}.${p}` : `${path}[${p.toString()}]`
      return augmentSetProp((target as any)[p], onSet, subpath, target)
    },
    set: (target: T, p: string | number | symbol, value: any): boolean => {
      const subpath = typeof p === 'string' ? `${path}.${p}` : `${path}[${p.toString()}]`;
      (target as any)[p] = value
      onSet(subpath)
      return true
    },
    apply: (target: T, thisArg: any, args: any[]): any => {
      const subpath = `${path}(...)`
      // Function might change stuff, so we call onSet (e.g. in arrays)
      // Worst case scenario we just call onSet when it's unnecessary
      const result = Reflect.apply(target as Function, thisArg, args)
      onSet(subpath)
      return result
    }
  })
}
