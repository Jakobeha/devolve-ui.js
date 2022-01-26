export module Generator {
  export function * range (min: number, max?: number): IterableIterator<number> {
    if (max === undefined) {
      max = min
      min = 0
    }

    for (let i = min; i <= max; i++) {
      yield i
    }
  }

  export function * filter<T> (input: Generator<T>, predicate: (input: T) => boolean): Generator<T> {
    for (const item of input) {
      if (predicate(item)) {
        yield item
      }
    }
  }

  export function * map<I, O> (input: Generator<I>, mapper: (input: I) => O): Generator<O> {
    for (const item of input) {
      yield mapper(item)
    }
  }

  export function * fromIter<T> (iterable: Iterable<T>): Generator<T> {
    for (const item of iterable) {
      yield item
    }
  }

  export function toArray<T> (generator: Generator<T>): T[] {
    const result: T[] = []
    for (const item of generator) {
      result.push(item)
    }
    return result
  }
}
