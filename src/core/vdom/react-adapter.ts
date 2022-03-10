import { VNode } from 'core/vdom/node'
import { intrinsics, JSXIntrinsics, VJSX } from 'core/vdom/jsx'
import { VComponent } from 'core/component'
import { IntoArray } from '@raycenity/misc-ts'

function createElement (
  element: undefined,
  props: {},
  ...children: VJSX[]
): VNode[]
function createElement <Key extends keyof JSXIntrinsics> (
  element: Key,
  props: Omit<JSXIntrinsics[Key], 'children'>,
  ...children: IntoArray<JSXIntrinsics[Key]['children']>
): VNode
function createElement <T extends VNode, Props, Children extends any[]> (
  element: (props: Props & { children?: Children }) => T,
  props: Props & { key?: string },
  ...children: Children
): VNode
function createElement <T extends VNode, Props extends { key?: string }, Children extends any[]> (
  element: undefined | keyof JSXIntrinsics | ((props: Props & { children?: Children }) => T),
  props: Props & { key?: string },
  ...children: Children
): VNode | VNode[] {
  // idk why jsx generates this code
  if (props === null || props === undefined) {
    // eslint-disable-next-line @typescript-eslint/consistent-type-assertions
    props = {} as Props
  }

  if (element === undefined) {
    // Fragment (<>{children}</>)
    return VJSX.collapse(children as VJSX[])
  } else if (typeof element === 'string') {
    // Intrinsic element
    const intrinsic = intrinsics[element]
    if (intrinsic === undefined) {
      throw new Error(`intrinsic element doesn't exist: ${element}`)
    } else {
      return intrinsic(props as any, ...children)
    }
  } else {
    // Component
    return VComponent(props.key ?? element.name, { ...props, children }, element)
  }
}

export const React = { createElement }
// @ts-expect-error
globalThis.React = React
