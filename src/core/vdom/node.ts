import { BoxAttrs, ColorAttrs, SourceAttrs, TextAttrs } from 'core/vdom/attrs'

export type VNode = VBox | VText | VColor | VSource

interface VNodeCommon {
  // Really don't want to use both null and undefined
  parent?: VBox | 'none'
}

export interface VBox extends BoxAttrs, VNodeCommon {
  type: 'box'
  children: VNode[]
}

export interface VText extends TextAttrs, VNodeCommon {
  type: 'text'
  text: string
}

export interface VColor extends ColorAttrs, VNodeCommon {
  type: 'color'
}

export interface VSource extends SourceAttrs, VNodeCommon {
  type: 'source'
}

export module VNode {
  export function isText (node: VNode): node is VText {
    return node.type === 'text'
  }

  export function isBox (node: VNode): node is VBox {
    return node.type === 'box'
  }

  export function isColor (node: VNode): node is VColor {
    return node.type === 'color'
  }

  export function isSource (node: VNode): node is VSource {
    return node.type === 'source'
  }

  export function convertInto<T extends VNode> (target: Partial<VNode>, newData: T): asserts target is T {
    for (const prop in target) {
      if (prop !== 'parent') {
        // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
        delete (target as any)[prop]
      }
    }
    for (const prop in newData) {
      if (prop === 'parent') {
        throw new Error('new data cannot have parent')
      } else {
        (target as T)[prop] = newData[prop]
        if (prop === 'children') {
          const children = (target as VBox).children
          for (const child of children) {
            child.parent = target as VBox
          }
        }
      }
    }
  }
}

export function VText (text: string, attrs: TextAttrs): VText {
  return { type: 'text', text, ...attrs }
}

export function VBox (children: VNode[], attrs: BoxAttrs): VBox {
  const box: VBox = { type: 'box', children, ...attrs }
  for (const child of box.children) {
    child.parent = box
  }
  return box
}

export function VColor (attrs: ColorAttrs): VColor {
  return { type: 'color', ...attrs }
}

export function VSource (attrs: SourceAttrs): VSource {
  return { type: 'source', ...attrs }
}
