export type BorderStyle =
  'single' |
  'card' |
  'double' |
  'rounded' |
  'dashed' |
  'thick' |
  'ascii' |
  'ascii-dashed'

export interface BorderAscii {
  topRight: string
  topLeft: string
  bottomRight: string
  bottomLeft: string
  top: string
  right: string
  bottom: string
  left: string
  topAlt?: string
  bottomAlt?: string
  leftAlt?: string
  rightAlt?: string
  leftAndMiddle: string
  topAndMiddle: string
  bottomAndMiddle: string
  rightAndMiddle: string
  middle: string
}

export module BorderStyle {
  export const ASCII: Record<BorderStyle, BorderAscii> = {
    single: asciiFromString(`
    ┌───┐
    │   │
    └───┘
    ├┬┴┤ ┼
    `),
    card: asciiFromString(`
    ╓───╖
    ║   ║
    ╙───╜
    ╟╥╨╢ ╫
    `),
    double: asciiFromString(`
    ╔═══╗
    ║   ║
    ╚═══╝
    ╠╦╩╣ ╬
    `),
    rounded: asciiFromString(`
    ╭───╮
    │   │
    ╰───╯
    ├┬┴┤ ┼
    `),
    dashed: asciiFromString(`
    ┌╌╌╌┐
    ╎   ╎
    └╌╌╌┘
    ├┬┴┤ ┼
    `),
    thick: asciiFromString(`
    ▛▀▀▀▜
    ▌   ▐
    ▙▄▄▄▟
    ▙▜▛▟ ▞
    `),
    ascii: asciiFromString(`
    +---+
    |   |
    +---+
    ++++ +
    `),
    'ascii-dashed': asciiFromString(`
    +- - -+
    |
          |
    + - - +
    +++ +
    `)
  }

  function asciiFromString (str: string): BorderAscii {
    const matrix = str.split('\n').map(row => row.trim()).filter(row => row.length > 0)
    switch (matrix[0].length) {
      case 5:
        return {
          topLeft: matrix[0][0],
          top: matrix[0][2],
          topRight: matrix[0][4],
          right: matrix[1][4],
          bottomRight: matrix[2][4],
          bottom: matrix[2][2],
          bottomLeft: matrix[2][0],
          left: matrix[1][0],
          leftAndMiddle: matrix[3][0],
          topAndMiddle: matrix[3][1],
          bottomAndMiddle: matrix[3][2],
          rightAndMiddle: matrix[3][3],
          middle: matrix[3][5]
        }
      case 7:
        return {
          topLeft: matrix[0][0],
          top: matrix[0][3],
          topAlt: matrix[0][4],
          topRight: matrix[0][6],
          right: matrix[1][6],
          rightAlt: matrix[2][6],
          bottomRight: matrix[3][6],
          bottomAlt: matrix[3][4],
          bottom: matrix[3][3],
          bottomLeft: matrix[3][0],
          leftAlt: matrix[2][0],
          left: matrix[1][0],
          leftAndMiddle: matrix[4][0],
          topAndMiddle: matrix[4][1],
          bottomAndMiddle: matrix[4][2],
          rightAndMiddle: matrix[4][3],
          middle: matrix[4][5]
        }
      default:
        throw new Error('Invalid matrix size')
    }
  }
}
