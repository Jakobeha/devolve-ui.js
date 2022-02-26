import { CommonAttrs, JSXBoxAttrs, JSXColorAttrs, JSXSourceAttrs, JSXTextAttrs } from 'core/vdom/attrs'
import { Bounds, BoundsSpec, SubLayout } from 'core/vdom/bounds'
import { VBox, VColor, VNode, VSource, VText } from 'core/vdom/node'
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
  source: JSXSourceAttrs & { children?: [] }
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

    return VBox(JSX.collapse(children), { bounds, visible, key, sublayout, ...attrs })
  },
  text: (props: JSXTextAttrs, ...text: string[]): VNode => VText(text.join(''), jsxToNormalAttrs(props)),
  color: (props: JSXColorAttrs): VNode => VColor(jsxToNormalAttrs(props)),
  source: (props: JSXSourceAttrs): VNode => VSource(jsxToNormalAttrs(props))
}

function jsxToNormalAttrs<T extends CommonAttrs> (jsxAttrs: T & BoundsSpec): Omit<T & BoundsSpec, 'bounds' | keyof BoundsSpec> & { bounds: Bounds } {
  const { x, y, z, anchorX, anchorY, width, height, bounds: explicitBounds, ...attrs } = jsxAttrs
  const bounds = explicitBounds ?? Bounds({ x, y, z, anchorX, anchorY, width, height })
  // @ts-expect-error this really should not be an error
  return { bounds, ...attrs }
}
