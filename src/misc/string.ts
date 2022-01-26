import stringWidth from 'string-width'

export module String {
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

  export function overlay (...strings: string[]): string {
    const liness = strings.map(string => string.split('\n'))
    const heights = liness.map(lines => lines.length)
    const widths = liness.map(lines => Math.max(...lines.map(stringWidth)))
    const resultLines = []
    for (let row = 0; row < Math.max(...heights); row++) {
      const resultLine = []
      const rows = liness.map(lines => lines[row] ?? '')
      for (let col = 0; col < Math.max(...widths); col++) {
        const chars = rows.map(row => row[col] ?? ' ')
        const topChar = chars.find(char => char !== ' ') ?? ' '
        resultLine.push(topChar)
      }
      resultLines.push(resultLine.join(''))
    }
    return resultLines.join('\n')
  }
}
