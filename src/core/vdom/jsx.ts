import { CommonAttrs, JSXBorderAttrs, JSXBoxAttrs, JSXColorAttrs, JSXSourceAttrs, JSXTextAttrs } from 'core/vdom/attrs'
import { Bounds, BoundsSpec, SubLayout } from 'core/vdom/bounds'
import { Color } from 'core/vdom/color'
import { VBorder, VBox, VColor, VNode, VSource, VText } from 'core/vdom/node'
import { ExplicitPartial, IntoArray } from '@raycenity/misc-ts'

export type JSX =
  VNode |
  null |
  undefined |
  JSX[]

export module JSX {
  export function collapse (jsx: JSX): VNode[] {
    if (Array.isArray(jsx)) {
      return jsx.flatMap(collapse)
    } else if (jsx === null || jsx === undefined) {
      return []
    } else {
      return [jsx]
    }
  }
}

export interface JSXIntrinsics {
  hbox: Omit<JSXBoxAttrs, 'direction'> & { children?: JSX[] }
  vbox: Omit<JSXBoxAttrs, 'direction'> & { children?: JSX[] }
  box: JSXBoxAttrs & { children?: JSX[] }
  text: JSXTextAttrs & { children?: string | string[] }
  color: JSXColorAttrs & { children?: [] }
  border: JSXBorderAttrs & { children?: [] }
  source: JSXSourceAttrs & { children?: [] }
}

export interface JSXIntrinsicAttributes {
  key?: string | number
}

export const intrinsics: {
  [Key in keyof JSXIntrinsics]: (props: Omit<JSXIntrinsics[Key], 'children'>, ...children: IntoArray<JSXIntrinsics[Key]['children']>) => VNode
} = {
  hbox: (props: Omit<JSXBoxAttrs, 'direction'>, ...children: JSX[]): VNode =>
    intrinsics.box({ ...props, direction: 'horizontal' }, ...children),
  vbox: (props: Omit<JSXBoxAttrs, 'direction'>, ...children: JSX[]): VNode =>
    intrinsics.box({ ...props, direction: 'vertical' }, ...children),
  box: (props: JSXBoxAttrs, ...children: JSX[]): VNode => {
    const { visible, key, bounds, direction, gap, custom, ...attrs } = jsxToNormalAttrs(props)
    const sublayout: ExplicitPartial<SubLayout> = { direction, gap, custom }

    const children_ = JSX.collapse(children)
    if (children_.length > 1 && direction === undefined) {
      console.warn('direction must be specified for multiple children')
    }

    return VBox(children_, { bounds, visible, key, sublayout, ...attrs })
  },
  text: (props: JSXTextAttrs, ...text: string[]): VNode => VText(text.join(''), jsxColorToNormalAttrs(props, false)),
  color: (props: JSXColorAttrs): VNode => VColor(jsxColorToNormalAttrs(props, true)),
  border: (props: JSXBorderAttrs): VNode => VBorder(jsxColorToNormalAttrs(props, false)),
  source: (props: JSXSourceAttrs): VNode => VSource(jsxToNormalAttrs(props))
}

function jsxToNormalAttrs<T extends CommonAttrs> (jsxAttrs: T & BoundsSpec): Omit<T & BoundsSpec, 'bounds' | keyof BoundsSpec> & { bounds: Bounds } {
  const { layout, x, y, z, anchorX, anchorY, width, height, bounds: explicitBounds, ...attrs } = jsxAttrs
  const bounds = explicitBounds ?? Bounds({ layout, x, y, z, anchorX, anchorY, width, height })
  return { bounds, ...attrs }
}

function jsxColorToNormalAttrs<T extends CommonAttrs & { color: Color | null }> (jsxAttrs: JSXColorAttrs<T>, requiresColor: boolean): T {
  const { color: colorSpec, name, red, green, blue, lightness, chroma, hue, bounds, ...attrs } = jsxToNormalAttrs(jsxAttrs)
  let color: Color | null = null
  if (colorSpec !== undefined) {
    color = Color(colorSpec)
  } else if (name !== undefined) {
    color = Color({ name })
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
