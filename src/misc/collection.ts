/** An iterable with a known size (similar to Collection from Kotlin) */
import { Iterable } from 'misc/iterable'
import { Arrays } from 'misc/array'
import { rand, Random } from 'misc/random'
import { Generator } from 'misc/generator'

export type Collection<T> = Iterable<T> & { length: number }

export function Collection <T> (items: Iterable<T>, length: number): Collection<T> {
  return {
    ...items,
    length
  }
}

export module Collection {
  export function range (start: number, end?: number): Collection<number> {
    if (end === undefined) {
      end = start
      start = 0
    }

    return Collection(Generator.range(start, end), end - start)
  }

  export function orNull<T> (items: Collection<T>): Collection<T | null> {
    return Collection(Iterable.orNull(items), items.length + 1)
  }

  // Top answer in https://stackoverflow.com/questions/11809502/which-is-better-way-to-calculate-ncr
  function nCr (n: number, r: number): number {
    if (r > n - r) r = n - r // because C(n, r) == C(n, n - r)
    let ans = 1

    for (let i = 1; i <= r; i++) {
      ans *= n - r + i
      ans /= i
    }

    return ans
  }

  export function permute <T> (items: Collection<T>, min?: number, max?: number): Collection<T[]> {
    max = max ?? min ?? items.length
    min = min ?? 0

    return Collection(
      Iterable.permute(items, min, max),
      Iterable.reduce(range(min, max), (acc, i) => acc + nCr(items.length, i), 0)
    )
  }

  export function indices <T> (items: Collection<T>): Collection<number> {
    return Collection(Iterable.indices(items), items.length)
  }

  export function atRandom <T> (items: Collection<T>, random?: Random): T {
    random = random ?? rand()
    const index = Math.floor(random() * items.length)
    return Iterable.atIndex(items, index)
  }

  export function takeRandom <T> (items: Collection<T>, max: number, random?: Random): T[] {
    random = random ?? rand()
    if (max >= items.length / 2) {
      // Just convert to array
      const array = Array.from(items)
      Arrays.shuffle(array, random)
      return array.slice(0, max)
    } else {
      // Pick n items at random keeping track of which ones have been picked
      const picked = new Set<number>()
      for (let i = 0; i < max; i++) {
        let index = Math.floor(random() * items.length)
        while (picked.has(index)) {
          index = Math.floor(random() * items.length)
        }
        picked.add(index)
      }

      const iterator = items[Symbol.iterator]()
      const result = []
      let lastIndex = 0
      for (const index of picked) {
        for (let i = lastIndex; i < index; i++) {
          const next = iterator.next()
          if (next.done === true) {
            return result
          }
        }
        const next = iterator.next()
        if (next.done === true) {
          return result
        }
        result.push(next.value)
        lastIndex = index
      }
      return result
    }
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
