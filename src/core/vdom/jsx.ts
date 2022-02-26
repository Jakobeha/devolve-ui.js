import { ColorAttrs, JSXBoxAttrs, SourceAttrs, TextAttrs } from 'core/vdom/attrs'
import { VBox, VColor, VNode, VSource, VText } from 'core/vdom/node'
import { IntoArray } from '@raycenity/misc-ts'

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
  text: TextAttrs & { children?: string | string[] }
  color: ColorAttrs & { children?: [] }
  source: SourceAttrs & { children?: [] }
}

export const intrinsics: {
  [Key in keyof JSXIntrinsics]: (props: Omit<JSXIntrinsics[Key], 'children'>, ...children: IntoArray<JSXIntrinsics[Key]['children']>) => VNode
} = {
  hbox: (props: Omit<JSXBoxAttrs, 'direction'>, ...children: JSX[]): VNode =>
    intrinsics.box({ ...props, direction: 'horizontal' }, ...children),
  vbox: (props: Omit<JSXBoxAttrs, 'direction'>, ...children: JSX[]): VNode =>
    intrinsics.box({ ...props, direction: 'vertical' }, ...children),
  box: (props: JSXBoxAttrs, ...children: JSX[]): VNode => {
    const { bounds, visible, key } = props
    delete props.bounds
    delete props.visible
    delete props.key
    // props becomes sublayout, since the rest of the props are sublayout

    return VBox(JSX.collapse(children), { bounds, visible, key, sublayout: props })
  },
  text: (props: TextAttrs, ...text: string[]): VNode => VText(text.join(''), props),
  color: (props: ColorAttrs): VNode => VColor(props),
  source: (props: SourceAttrs): VNode => VSource(props)
}
