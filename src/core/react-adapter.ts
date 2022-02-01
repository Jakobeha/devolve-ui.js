import { VJSX, VNode } from 'core/vdom'
import { VComponent } from 'core/component'
import { JSX } from 'jsx-runtime'
import { elements, HBox, YBox } from 'core/elements'
import { Strings } from 'misc'

function createElement (
  element: undefined,
  props: {},
  ...children: VJSX[]
): VNode[]
function createElement (
  element: keyof JSX.IntrinsicElements,
  props: any,
  ...children: VJSX[]
): VNode
function createElement <T extends VNode, Props, Children extends any[]> (
  element: (props: Props & { children?: Children}) => T,
  props: Props,
  ...children: Children
): VNode
function createElement <T extends VNode, Props, Children extends any[]> (
  element: undefined | keyof JSX.IntrinsicElements | ((props: Props & { children?: Children}) => T),
  props: Props,
  ...children: Children | VJSX[]
): VNode | VNode[] {
  // idk why jsx generates this code
  if (props === null || props === undefined) {
    // eslint-disable-next-line @typescript-eslint/consistent-type-assertions
    props = {} as Props
  }

  if (element === undefined) {
    return VJSX.collapse(children as VJSX[])
  } else if (typeof element === 'string') {
    return getIntrinsicFunction(element)(props, ...children)
  } else {
    return VComponent(() => element({ ...props, children: children as Children }))
  }
}

export const React = {
  createElement
}
// @ts-expect-error
globalThis.React = React

function getIntrinsicFunction (element: keyof JSX.IntrinsicElements): (props: any, children?: any) => VNode {
  switch (element) {
    case 'hbox':
      return HBox
    case 'vbox':
      return YBox
    default: {
      const intrinsic = Object.entries(elements).find(([name, _]) => Strings.uncapitalize(name) === element)?.[1]
      if (intrinsic !== undefined) {
        return intrinsic
      } else {
        throw new Error(`Unknown element: ${element as string}`)
      }
    }
  }
}
