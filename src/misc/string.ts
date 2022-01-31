import stringWidth from 'string-width'

export module Strings {
  // From https://github.com/chalk/ansi-regex/blob/main/index.js
  function ansiRegex ({ onlyStart = false, global = false } = {}): RegExp {
    const pattern = [
      (onlyStart ? '^' : '') + '[\\u001B\\u009B][[\\]()#;?]*(?:(?:(?:(?:;[-a-zA-Z\\d\\/#&.:=?%@~_]+)*|[a-zA-Z\\d]+(?:;[-a-zA-Z\\d\\/#&.:=?%@~_]*)*)?\\u0007)',
      '(?:(?:\\d{1,4}(?:;\\d{0,4})*)?[\\dA-PR-TZcf-nq-uy=><~]))'
    ].join('|')

    return new RegExp(pattern, global ? 'g' : '')
  }
  const ANSI_REGEX_ONLY_START = ansiRegex({ onlyStart: true })

  export function chunk (string: string, size: number): string[] {
    const result: string[] = []
    for (let i = 0; i < string.length; i += size) {
      result.push(string.substring(i, size))
    }
    return result
  }

  /** Takes into account the real string monospace width, AKA multi-width characters and ANSI escape codes. */
  export function padStartSmart (base: string, width: number, padding: string = ' '): string {
    let baseWidth = stringWidth(base)
    const paddingWidth = stringWidth(padding)
    while (baseWidth < width) {
      base = padding + base
      baseWidth += paddingWidth
    }
    return base
  }

  /** Takes into account the real string monospace width, AKA multi-width characters and ANSI escape codes. */
  export function padEndSmart (base: string, width: number, padding: string = ' '): string {
    let baseWidth = stringWidth(base)
    const paddingWidth = stringWidth(padding)
    while (baseWidth < width) {
      base += padding
      baseWidth += paddingWidth
    }
    return base
  }

  export function uncapitalize<S extends string> (string: S): Uncapitalize<S> {
    return string.charAt(0).toLowerCase() + string.slice(1) as Uncapitalize<S>
  }

  export function overlay (...liness: string[][]): string[] {
    if (liness.length === 1) {
      return liness[0]
    }

    const heights = liness.map(lines => lines.length)
    const widths = liness.map(lines => Math.max(...lines.map(stringWidth)))

    const offsetss = liness.map(lines => lines.map(() => 0))
    const resultLines = []
    for (let row = 0; row < Math.max(...heights); row++) {
      let resultLine = ''
      const rows = liness.map(lines => lines[row] ?? '')
      const offsets = offsetss.map(offsets => offsets[row] ?? 0)
      for (let col = 0; col < Math.max(...widths);) {
        const chars = rows.map((row, i) => row[offsets[i]] ?? ' ')

        let hasZeroWidthChars = true
        while (hasZeroWidthChars) {
          hasZeroWidthChars = false
          for (let i = 0; i < chars.length; i++) {
            const ansiEscape = chars[i].match(ANSI_REGEX_ONLY_START)
            if (ansiEscape !== null) {
              // Count the entire ansi-escape as a zero-width
              resultLine += ansiEscape[0]
              offsets[i] += ansiEscape[0].length
              chars[i] = rows[i][offsets[i]] ?? ' '
              hasZeroWidthChars = true
            } else if (stringWidth(chars[i]) === 0) {
              resultLine += chars[i]
              offsets[i]++
              chars[i] = rows[i][offsets[i]] ?? ' '
              hasZeroWidthChars = true
            }
          }
        }

        const topChar = chars.find(char => char !== ' ') ?? ' '
        resultLine += topChar

        const topCharLen = stringWidth(topChar)
        col += topCharLen
        for (let i = 0; i < offsets.length; i++) {
          // Not going to deal with interleaved multi-width / zero-width characters
          offsets[i] += topCharLen
        }
      }
      resultLines.push(resultLine)
    }
    return resultLines
  }
}
