import { VNode } from 'core/vdom'
import { VComponent } from 'core/component'
import { JSX } from 'jsx-runtime'
import { Box, HBox, Image, Text, YBox } from 'core/elements'

export function createElement<T extends VNode, Props, Children> (
  element: keyof JSX.IntrinsicElements | ((props: Props, children?: Children) => T),
  props: Props,
  children: Children
): VNode {
  const func = typeof element === 'string' ? getIntrinsicFunction(element) : element
  return VComponent(() => func(props, children))
}

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
  }
}
