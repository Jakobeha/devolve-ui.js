// From https://stackoverflow.com/questions/521295/seeding-the-random-number-generator-in-javascript
function xmur3 (str: string): () => number {
  let i, h: number
  for (i = 0, h = 1779033703 ^ str.length; i < str.length; i++) {
    h = Math.imul(h ^ str.charCodeAt(i), 3432918353)
    h = h << 13 | h >>> 19
  }
  return () => {
    h = Math.imul(h ^ h >>> 16, 2246822507)
    h = Math.imul(h ^ h >>> 13, 3266489909)
    return (h ^= h >>> 16) >>> 0
  }
}

function mulberry32 (a: number): { func: () => number, getA: () => number } {
  return {
    func: () => {
      let t = a += 0x6D2B79F5
      t = Math.imul(t ^ t >>> 15, t | 1)
      t ^= t + Math.imul(t ^ t >>> 7, t | 61)
      return ((t ^ t >>> 14) >>> 0) / 4294967296
    },
    getA: () => a
  }
}

export interface Random {
  (): number

  /**
   * Create an exact copy of the random seed with this exact state.
   * Any results returned by the copy will be returned in the same order by the original and vice versa.
   * This may throw an error if you don't have access to that information.
   */
  copy: () => Random
  /** Advance the state and return a completely different random seeded on the current state */
  branch: () => Random
}

export function rand (seed: string = Date.now().toString()): Random {
  return _rand(xmur3(seed)())
}

export function _rand (seed: number): Random {
  const { func, getA } = mulberry32(seed)
  return Object.assign(func, {
    copy (): Random {
      return _rand(getA())
    },
    branch (): Random {
      return rand(`${seed}#${func()}`)
    }
  })
}
