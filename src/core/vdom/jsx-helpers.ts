import { CommonAttrs, JSXColorAttrs } from 'core/vdom/attrs'
import { Bounds, BoundsSpec } from 'core/vdom/bounds'
import { Color } from 'core/vdom/color'

export function jsxToNormalAttrs<T extends CommonAttrs> (jsxAttrs: T & BoundsSpec): Omit<T & BoundsSpec, 'bounds' | keyof BoundsSpec> & { bounds: Bounds } {
  const { layout, x, y, z, anchorX, anchorY, width, height, bounds: explicitBounds, ...attrs } = jsxAttrs
  const bounds = explicitBounds ?? Bounds({ layout, x, y, z, anchorX, anchorY, width, height })
  return { bounds, ...attrs }
}

export function jsxColorToNormalAttrs<T extends CommonAttrs & { color: Color | null }> (jsxAttrs: JSXColorAttrs<T>, requiresColor: boolean): T {
  const { color: colorSpec, red, green, blue, lightness, chroma, hue, bounds, ...attrs } = jsxToNormalAttrs(jsxAttrs)
  let color: Color | null = null
  if (colorSpec !== undefined) {
    color = Color(colorSpec)
  } else if (red !== undefined && green !== undefined && blue !== undefined) {
    color = Color({ red, green, blue })
  } else if (lightness !== undefined && chroma !== undefined && hue !== undefined) {
    color = Color({ lightness, chroma, hue })
  }
  if (color === null && requiresColor) {
    throw new Error(`Can't deduce color: ${JSON.stringify(jsxAttrs)}`)
  }
  // eslint-disable-next-line @typescript-eslint/consistent-type-assertions
  return { color, bounds, ...attrs } as T
}
