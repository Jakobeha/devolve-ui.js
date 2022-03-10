
export const TRANSPARENT = '\u{FFF0}'

export type CharColor = string
export type CharColorType = 'fg' | 'bg'

export function CharColor (type: CharColorType, openEscape: string, closeEscape: string): string {
  switch (type) {
    case 'fg':
      return `\u{FFF1}${openEscape}\u{FFF2}${closeEscape}`
    case 'bg':
      return `\u{FFF3}${openEscape}\u{FFF4}${closeEscape}`
  }
}

export module CharColor {
  export function has (type: CharColorType, string: string): boolean {
    switch (type) {
      case 'fg':
        return string.includes('\u{FFF1}')
      case 'bg':
        return string.includes('\u{FFF3}')
    }
  }

  export function get (type: CharColorType, string: string): CharColor | null {
    const fgIndex = string.indexOf('\u{FFF1}')
    const bgIndex = string.indexOf('\u{FFF3}')
    switch (type) {
      case 'fg':
        if (fgIndex === -1) {
          return null
        } else if (bgIndex < fgIndex) {
          return string.substring(fgIndex)
        } else {
          return string.substring(fgIndex, bgIndex)
        }
      case 'bg':
        if (bgIndex === -1) {
          return null
        } else if (fgIndex < bgIndex) {
          return string.substring(bgIndex)
        } else {
          return string.substring(bgIndex, fgIndex)
        }
    }
  }

  export function remove (string: string): string {
    // noinspection DuplicatedCode
    const fgIndex = string.indexOf('\u{FFF1}')
    const bgIndex = string.indexOf('\u{FFF3}')
    if (fgIndex === -1 && bgIndex === -1) {
      return string
    } else if (fgIndex === -1) {
      return string.substring(0, bgIndex)
    } else if (bgIndex === -1) {
      return string.substring(0, fgIndex)
    } else {
      return string.substring(0, Math.min(fgIndex, bgIndex))
    }
  }

  export function open (color: CharColor): string {
    // noinspection DuplicatedCode
    const fgIndex = color.indexOf('\u{FFF1}')
    const bgIndex = color.indexOf('\u{FFF3}')
    let open = ''
    if (fgIndex !== -1) {
      open += color.substring(fgIndex + 1, color.indexOf('\u{FFF2}'))
    }
    if (bgIndex !== -1) {
      open += color.substring(bgIndex + 1, color.indexOf('\u{FFF4}'))
    }
    return open
  }

  export function close (color: CharColor): string {
    // noinspection DuplicatedCode
    const fgIndex = color.indexOf('\u{FFF2}')
    const bgIndex = color.indexOf('\u{FFF4}')
    let close = ''
    if (fgIndex !== -1) {
      close += color.substring(fgIndex + 1, bgIndex > fgIndex ? color.indexOf('\u{FFF3}') : color.length)
    }
    if (bgIndex !== -1) {
      close += color.substring(bgIndex + 1, fgIndex > bgIndex ? color.indexOf('\u{FFF1}') : color.length)
    }
    return close
  }
}
