import { VJSX, VNode } from 'core/vdom'
import { VComponent } from 'core/component'

export function createComponent<T extends VNode, Props> (
  element: (props: Props, children?: VJSX) => T,
  props: Props,
  children: VJSX
): VNode {
  return VComponent(() => element(props, children))
}
