import { BoxAttrs, ImageAttrs } from 'node-agnostic'

export type VNode = VText | VBox | VImage
export type VJSX =
  VNode |
  null |
  undefined |
  VJSX[]

interface VNodeCommon {
  parent: VBox | null
}

export interface VText extends VNodeCommon {
  text: string
}

export interface VBox extends VNodeCommon {
  box: BoxAttrs
  children: VNode[]
}

export interface VImage extends VNodeCommon {
  image: ImageAttrs
  path: string
}

export module VJSX {
  export function collapse (jsx: VJSX): VNode[] {
    if (Array.isArray(jsx)) {
      return jsx.flatMap(collapse)
    } else if (jsx === null || jsx === undefined) {
      return []
    } else {
      return [jsx]
    }
  }
}

export module VNode {
  export function isText (node: VNode): node is VText {
    return 'text' in node && node.text !== undefined
  }

  export function isBox (node: VNode): node is VBox {
    return 'box' in node && node.box !== undefined
  }

  export function isImage (node: VNode): node is VImage {
    return 'image' in node && node.image !== undefined
  }

  export function convertInto<T extends VNode> (target: Partial<VNode>, newData: T): asserts target is T {
    for (const prop in target) {
      if (prop in newData) {
        // @ts-expect-error
        target[prop] = newData[prop]
      } else if (prop !== 'renderer') {
        // @ts-expect-error
        // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
        delete target[prop]
      }
    }
  }

  export function type (node: VNode): string {
    if (isText(node)) {
      return 'text'
    } else if (isBox(node)) {
      return 'box'
    } else if (isImage(node)) {
      return 'image'
    } else {
      throw new Error('Unknown node type')
    }
  }
}

export function VText (text: string): VText {
  return { text, parent: null }
}

export module VText {

}

export function VBox (children: VNode[], props: BoxAttrs = {}): VBox {
  const box = {
    box: props,
    children,
    parent: null
  }
  for (const child of children) {
    if (child.parent !== null) {
      throw new Error('Child already has a parent')
    }
    child.parent = box
  }
  return box
}

export function VImage (path: string, props: BoxAttrs = {}): VImage {
  return {
    image: props,
    path,
    parent: null
  }
}
