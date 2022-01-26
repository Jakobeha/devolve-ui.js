import { Random } from 'misc/random'

export function range (start: number, end?: number, step?: number): number[] {
  if (end === undefined) {
    end = start
    start = 0
  }
  if (step === undefined) {
    step = 1
  }

  if ((start !== end && step === 0) || (start > end) === (step > 0)) {
    throw new Error('infinite range detected and range is not lazy')
  }

  const result: number[] = []
  for (let i = start; i < end; i += step) {
    result.push(i)
  }
  return result
}

export module Arrays {
  // https://stackoverflow.com/questions/2450954/how-to-randomize-shuffle-a-javascript-array
  export function shuffle<T> (array: T[], random: Random): void {
    for (let i = array.length - 1; i > 0; i--) {
      const j = Math.floor(random() * (i + 1));
      [array[i], array[j]] = [array[j], array[i]]
    }
  }

  export function mapNotNull<I, O> (array: I[], transform: (input: I) => O | null | undefined): O[] {
    const result: O[] = []
    for (const item of array) {
      const transformed = transform(item)
      if (transformed != null) {
        result.push(transformed)
      }
    }
    return result
  }
}
