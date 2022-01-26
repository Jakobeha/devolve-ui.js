// Derived from the example in https://github.com/solidjs/solid/tree/main/packages/solid/universal/README.md
import { createRenderer } from 'solid-js/universal'
import { VElement, VNode, VText } from 'cli/vdom'
import { TerminalContainer, TerminalRenderer, TerminalRendererImpl } from 'cli/renderer'
import { Renderer } from 'universal'

export const renderer: Renderer<VNode> = createRenderer<VNode>({
  createElement(tag) {
    return VElement(tag)
  },
  createTextNode(value) {
    return VText(value)
  },
  replaceText(textNode, value) {
    VNode.setText(textNode, value)
  },
  setProperty(node, name, value) {
    VNode.setProperty(node, name, value)
  },
  insertNode(parent, node, anchor) {
    VNode.insertChild(parent, node, anchor)
  },
  removeNode(parent, node) {
    VNode.removeChild(parent, node)
  },
  isTextNode(node) {
    return VNode.isText(node)
  },
  getParentNode(node) {
    return VNode.getParent(node)
  },
  getFirstChild(node) {
    return VNode.getChildren(node)[0]
  },
  getNextSibling(node) {
    // This has to be mistyped: how does the renderer know when there are no more children if this can't return null?
    // So we allow it to silently return null
    return VNode.getNextSibling(node)!
  }
})

export function render(template: () => VNode, container?: TerminalContainer): TerminalRenderer {
  const renderer = new TerminalRendererImpl(container)
  renderer.start()
  return renderer
}

export const {
  effect,
  createComponent,
  memo,
  insert,
  spread,
  mergeProps
} = renderer
