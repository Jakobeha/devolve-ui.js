import { BoxAttrs, BoxAttrsWithoutDirection, ColorAttrs, SourceAttrs, TextAttrs } from 'core/vdom/attrs'
import { VBox, VColor, VNode, VSource, VText } from 'core/vdom/node'

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
  hbox: BoxAttrsWithoutDirection & { children: JSX[] }
  vbox: BoxAttrsWithoutDirection & { children: JSX[] }
  box: BoxAttrs & { children: JSX[] }
  text: TextAttrs & { children: [string] }
  color: ColorAttrs & { children: [] }
  source: SourceAttrs & { children: [] }
}

export const intrinsics: {
  [Key in keyof JSXIntrinsics]: (props: Omit<JSXIntrinsics[Key], 'children'>, ...children: JSXIntrinsics[Key]['children']) => VNode
} = {
  hbox: (props: BoxAttrsWithoutDirection, ...children: JSX[]): VNode =>
    intrinsics.box({ ...props, sublayout: { ...(props.sublayout ?? {}), direction: 'horizontal' } }, ...children),
  vbox: (props: BoxAttrsWithoutDirection, ...children: JSX[]): VNode =>
    intrinsics.box({ ...props, sublayout: { ...(props.sublayout ?? {}), direction: 'vertical' } }, ...children),
  box: (props: BoxAttrs, ...children: JSX[]): VNode => VBox(JSX.collapse(children) as VNode[], props),
  text: (props: TextAttrs, text: string): VNode => VText(text, props),
  color: (props: ColorAttrs): VNode => VColor(props),
  source: (props: SourceAttrs): VNode => VSource(props)
}
