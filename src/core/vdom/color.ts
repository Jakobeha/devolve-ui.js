import { Color as W3Color, gam_sRGB, Lab_to_XYZ, LCH_to_Lab, XYZ_to_lin_sRGB } from 'core/vdom/w3-color-conversions'

export interface LCHColor {
  lightness: number
  chroma: number
  hue: number
  alpha?: number
}

export interface RGBColor {
  red: number
  green: number
  blue: number
  alpha?: number
}

export type HexColor = `#${string}`

export type ColorName =
  'red' |
  'orange' |
  'gold' |
  'yellow' |
  'green' |
  'blue' |
  'purple' |
  'pink' |
  'brown' |
  'black' |
  'white' |
  'gray' |
  'cyan'

export type Color =
  LCHColor |
  RGBColor

export type ColorSpec =
  Color |
  HexColor |
  { name: ColorName } |
  ColorName

export function Color (color: ColorSpec): RGBColor | LCHColor {
  if (typeof color === 'string') {
    if (color.startsWith('#')) {
      if (!/^#[0-9A-F]+$/i.test(color)) {
        throw new Error(`Invalid hex color: ${color}`)
      }
      switch (color.length) {
        case 4:
        case 5:
          return {
            red: (parseInt(color[1], 16) * 17) / 255,
            green: (parseInt(color[2], 16) * 17) / 255,
            blue: parseInt(color[3], 16) / 15,
            alpha: color.length === 5 ? parseInt(color[4], 16) / 15 : 1
          }
        case 7:
        case 9:
          return {
            red: parseInt(color[1] + color[2], 16) / 255,
            green: parseInt(color[3] + color[4], 16) / 255,
            blue: parseInt(color[5] + color[6], 16) / 255,
            alpha: color.length === 9 ? parseInt(color[7] + color[8], 16) / 255 : 1
          }
        case 13:
        case 17:
          return {
            red: parseInt(color[1] + color[2] + color[3] + color[4], 16) / 65205,
            green: parseInt(color[5] + color[6] + color[7] + color[8], 16) / 65205,
            blue: parseInt(color[9] + color[10] + color[11] + color[12], 16) / 65205,
            alpha: color.length === 17 ? parseInt(color[13] + color[14] + color[15] + color[16], 16) / 65205 : 1
          }
        default:
          throw new Error(`Invalid hex color length: ${color}`)
      }
    } else {
      const result = LCHColor.FROM_STRING[color as ColorName]
      if (result === undefined) {
        throw new Error(`Unknown color: ${color}`)
      }
      return result
    }
  } else if ('name' in color) {
    const result = LCHColor.FROM_STRING[color.name]
    if (result === undefined) {
      throw new Error(`Unknown color: ${JSON.stringify(color)}`)
    }
    return result
  } else {
    return color
  }
}

export module LCHColor {
  export function toRGB (color: LCHColor): RGBColor {
    if (color.chroma === 0) {
      // Easy case
      // Also the other case doesn't actually return the same as this because our converter is broken :)
      return {
        red: color.lightness / 100,
        green: color.lightness / 100,
        blue: color.lightness / 100,
        alpha: color.alpha
      }
    } else {
      const lch: W3Color = [color.lightness, color.chroma, color.hue]
      const [red, green, blue] = gam_sRGB(XYZ_to_lin_sRGB(Lab_to_XYZ(LCH_to_Lab(lch))))

      if (red < 0 || red > 1 || green < 0 || green > 1 || blue < 0 || blue > 1) {
        // Raw rgb values may actually be over or under 0 and 1,
        // if so we need to auto-correct them
        // unfortunately I have no idea how except to lower chroma and retry :(
        if (color.chroma > 0.1) {
          return toRGB({ ...color, chroma: color.chroma * 0.9 })
        } else {
          return toRGB({ ...color, chroma: 0 })
        }
      }

      return {
        red,
        green,
        blue,
        alpha: color.alpha
      }
    }
  }

  export const FROM_STRING: Record<ColorName, LCHColor> = {
    red: { lightness: 50, chroma: 75, hue: 25 },
    orange: { lightness: 50, chroma: 75, hue: 55 },
    gold: { lightness: 50, chroma: 75, hue: 80 },
    green: { lightness: 50, chroma: 75, hue: 135 },
    blue: { lightness: 50, chroma: 75, hue: 280 },
    purple: { lightness: 50, chroma: 75, hue: 320 },
    pink: { lightness: 50, chroma: 75, hue: 0 },
    black: { lightness: 0, chroma: 0, hue: 0 },
    white: { lightness: 100, chroma: 0, hue: 0 },
    gray: { lightness: 50, chroma: 0, hue: 0 },
    cyan: { lightness: 50, chroma: 75, hue: 200 },
    yellow: { lightness: 75, chroma: 75, hue: 80 },
    brown: { lightness: 35, chroma: 75, hue: 80 }
  }
}

export module RGBColor {
  export function toLCH (color: RGBColor): LCHColor {
    const { red, green, blue } = color
    const lightness = (red + green + blue) / 3
    const chroma = Math.sqrt(red * red + green * green + blue * blue)
    const hue = Math.atan2(green - blue, red - green) * 180 / Math.PI
    return { lightness, chroma, hue }
  }
}

export module Color {
  export function toRGB (color: Color): RGBColor {
    if ('red' in color && 'green' in color && 'blue' in color) {
      return color
    } else if ('lightness' in color && 'chroma' in color && 'hue' in color) {
      return LCHColor.toRGB(color)
    } else {
      throw new Error(`Invalid color: ${JSON.stringify(color)}`)
    }
  }

  export function toLCH (color: Color): LCHColor {
    if ('lightness' in color && 'chroma' in color && 'hue' in color) {
      return color
    } else if ('red' in color && 'green' in color && 'blue' in color) {
      return RGBColor.toLCH(color)
    } else {
      throw new Error(`Invalid color: ${JSON.stringify(color)}`)
    }
  }
}
