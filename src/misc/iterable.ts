import { Generator } from 'misc/generator'

export module Iterable {
  export function * orNull<T> (items: Iterable<T>): Generator<T | null> {
    yield null
    for (const item of items) {
      yield item
    }
  }

  export function * permute <T> (items: Iterable<T>, min?: number, max?: number): Generator<T[]> {
    const array = toArray(items)
    max = max ?? min ?? array.length
    min = min ?? 0

    for (let len = min; len <= max; len++) {
      yield * permute2(array, len)
    }
  }

  function * permute2 <T> (array: T[], len: number): Generator<T[]> {
    if (len === 0) {
      yield []
    } else {
      for (let i = 0; i < len; i++) {
        for (const perm of permute2(array.slice(i + 1), len - 1)) {
          yield [array[i], ...perm]
        }
      }
    }
  }

  export function * indices <T> (items: Iterable<T>): Generator<number> {
    let i = 0
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    for (const _ of items) {
      yield i++
    }
  }

  export function atIndex <T> (items: Iterable<T>, index: number): T {
    const result = atIndexOrNull(items, index)
    if (result === null) {
      throw new Error(`Index out of bounds: ${index}`)
    }
    return result
  }

  export function atIndexOrNull <T> (items: Iterable<T>, index: number): T | null {
    let i = 0
    for (const item of items) {
      if (i === index) {
        return item
      }
      i++
    }
    return null
  }

  export function take <T> (items: Iterable<T>, max: number): T[] {
    if (items instanceof Array) {
      return items.slice(0, max)
    }

    const array = []
    for (const item of items) {
      if (array.length === max) {
        break
      }
      array.push(item)
    }
    return array
  }

  export function reduce <T, R> (items: Iterable<T>, reduce: (acc: R, item: T) => R, acc: R): R {
    for (const item of items) {
      acc = reduce(acc, item)
    }
    return acc
  }

  export function toArray <T> (items: Iterable<T>): T[] {
    if (items instanceof Array) {
      return items
    }

    const array = []
    for (const item of items) {
      array.push(item)
    }
    return array
  }
}
