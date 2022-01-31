import { VJSX, VNode } from 'core/vdom'
import { VComponent } from 'core/component'
import { JSX } from 'jsx-runtime'
import { Box, HBox, Image, Text, YBox } from 'core/elements'

function createElement (
  element: undefined,
  props: {},
  children: VJSX | VJSX[],
): VNode[]
function createElement <T extends VNode, Props, Children> (
  element: keyof JSX.IntrinsicElements | ((props: Props, children?: Children) => T),
  props: Props,
  children: Children
): VNode
function createElement <T extends VNode, Props, Children> (
  element: undefined | keyof JSX.IntrinsicElements | ((props: Props, children?: Children) => T),
  props: Props,
  children: Children | VJSX | VJSX[]
): VNode | VNode[] {
  // idk why jsx generates this code
  if (props === null || props === undefined) {
    // eslint-disable-next-line @typescript-eslint/consistent-type-assertions
    props = {} as Props
  }

  if (element === undefined) {
    return VJSX.collapse(children as VJSX[])
  } else if (typeof element === 'string') {
    return getIntrinsicFunction(element)(props, children)
  } else {
    return VComponent(() => element(props, children as Children))
  }
}

export const React = {
  createElement
}
// @ts-expect-error
globalThis.React = React

function getIntrinsicFunction (element: keyof JSX.IntrinsicElements): (props: any, children?: any) => VNode {
  switch (element) {
    case 'box':
      return Box
    case 'hbox':
      return HBox
    case 'vbox':
      return YBox
    case 'image':
      return Image
    case 'text':
      return Text
    default:
      throw new Error(`Unknown element: ${element as string}`)
  }
}
