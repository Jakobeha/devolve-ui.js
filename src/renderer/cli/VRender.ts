import { BoundingBox } from 'core'
import { CharColor, TRANSPARENT } from 'renderer/cli/CharColor'

/**
 * Each x/y index represents the character at that exact position in the terminal.
 * If the character is multi-width, then the next character will be empty.
 * If the character is \u{FFF0} it is transparent (the character under will be used).
 * If the character contains \u{FFF1} and \u{FFF2}, it is a background (characters above will also have the background unless they also contain a background)
 */
export type VRender = string[][]

export module VRender {
  export function addColor (vrender: VRender, color: CharColor): void {
    for (const line of vrender) {
      for (let x = 0; x < line.length; x++) {
        line[x] += color
      }
    }
  }

  export function translate1 (vrender: VRender, bounds: BoundingBox): void {
    const width = bounds.width ?? getWidth(vrender)
    const height = bounds.height ?? getHeight(vrender)

    const xOffset = bounds.x - (bounds.anchorX * width)
    const yOffset = bounds.y - (bounds.anchorY * height)

    return translate2(vrender, xOffset, yOffset)
  }

  export function translate2 (vrender: VRender, xOffset: number, yOffset: number): void {
    xOffset = Math.round(xOffset)
    yOffset = Math.round(yOffset)

    for (const line of vrender) {
      if (line.length > 0) {
        for (let x = 0; x < xOffset; x++) {
          line.unshift(TRANSPARENT)
        }
      }
    }
    for (let y = 0; y < yOffset; y++) {
      vrender.unshift([])
    }
  }

  export function collapse (textMatrix: Record<number, VRender>): string[][] {
    for (const key of Object.keys(textMatrix)) {
      if (isNaN(parseFloat(key))) {
        delete textMatrix[key as any]
      }
    }

    if (Object.values(textMatrix).length === 0) {
      return []
    }

    // Array length not width
    const length = Math.max(...Object.values(textMatrix).map(getWidth))
    const height = Math.max(...Object.values(textMatrix).map(getHeight))
    const matrixSorted = Object.entries(textMatrix).sort(([lhs], [rhs]) => Number(rhs) - Number(lhs)).map(([, lines]) => lines)

    const result: string[][] = Array(height).fill(null).map(() => Array(length).fill(TRANSPARENT))
    for (const lines of matrixSorted) {
      for (let y = 0; y < lines.length; y++) {
        const line = lines[y]
        const resultLine = result[y]
        for (let x = 0; x < line.length; x++) {
          const resultChar = resultLine[x]
          const char = line[x]
          if (resultChar === TRANSPARENT) {
            // fall through
            resultLine[x] = char
          } else if (!CharColor.has('bg', resultChar) && CharColor.has('bg', char)) {
            // add color
            resultLine[x] += CharColor.get('bg', char)!
          }
        }
      }
    }
    for (let y = 0; y < result.length; y++) {
      const line = result[y]
      let prevFg: CharColor | null = null
      let prevBg: CharColor | null = null
      for (let x = 0; x < line.length; x++) {
        const char = line[x]

        // Fill if fallthrough
        if (char === TRANSPARENT) {
          line[x] = ' '
        }

        // Add open or close for color
        const fg = CharColor.get('fg', char)
        const bg = CharColor.get('bg', char)
        if (fg !== null || bg !== null) {
          line[x] = CharColor.remove(line[x])
        }
        if (prevFg !== fg && fg !== null) {
          line[x] = CharColor.open(fg) + line[x]
        }
        if (prevBg !== bg && bg !== null) {
          line[x] = CharColor.open(bg) + line[x]
        }
        if (prevBg !== bg && prevBg !== null) {
          line[x] = CharColor.close(prevBg) + line[x]
        }
        if (prevFg !== fg && prevFg !== null) {
          line[x] = CharColor.close(prevFg) + line[x]
        }
        prevFg = fg
        prevBg = bg
      }

      if (prevBg !== null) {
        line[line.length - 1] += CharColor.close(prevBg)
      }
      if (prevFg !== null) {
        line[line.length - 1] += CharColor.close(prevFg)
      }
    }
    return result
  }

  function getWidth (vrender: VRender): number {
    return Math.max(0, ...vrender.map(line => line.length))
  }

  function getHeight (vrender: VRender): number {
    return vrender.length
  }
}
