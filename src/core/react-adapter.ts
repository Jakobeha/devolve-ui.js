// Derived from the example in https://github.com/solidjs/solid/tree/main/packages/solid/universal/README.md
import { VComponent, VJSX, VNode } from 'core/vdom'

export function createComponent<T extends VNode, Props> (
  element: (props: Props & { children?: VJSX }) => T,
  props: Props,
  children: VJSX
): VNode {
  return VComponent(() => element({ ...props, children }))
}
