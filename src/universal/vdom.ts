import { RendererImpl } from 'universal/renderer'
import { BoxAttrs, ImageAttrs } from 'types'

export type VNode = VText | VElement
export type VJSX =
  VNode |
  null |
  undefined |
  VJSX[] |
  (() => VJSX)

interface VNodeCommon {
  parent: VElement | null
  renderer: RendererImpl<any, any> | null
}

export interface VText extends VNodeCommon {
  text: string
}

export type VTag = 'box' | 'image'

export interface VProps extends BoxAttrs, ImageAttrs {}

export interface VElement<Tag extends VTag = VTag> extends VNodeCommon {
  tag: Tag
  props: VProps
  children: VNode[]
}

export module VJSX {
  export function collapse(jsx: VJSX): VNode[] {
    if (Array.isArray(jsx)) {
      return jsx.flatMap(collapse)
    } else if (typeof jsx === 'function') {
      return collapse(jsx())
    } else if (jsx === null || jsx === undefined) {
      return []
    } else {
      return [jsx]
    }
  }
}

export module VNode {
  export function isText(node: VNode): node is VText {
    return 'text' in node
  }

  export function isElement(node: VNode, tag?: string): node is VElement {
    return 'tag' in node && (tag === undefined || node.tag === tag)
  }

  export function convertIntoElement(textNode: VText): VElement {
    const text = textNode.text
    const textChild = VText(text)
    const newNode = textNode as unknown as Partial<VElement & VText>

    delete newNode.text
    Object.assign(newNode, {
      tag: 'box',
      props: {},
      children: [textChild]
    } as VElement)
    return newNode as VElement
  }

  export function type(node: VNode): string {
    if (isText(node)) {
      return 'text'
    } else {
      return node.tag
    }
  }

  export function setText(node: VNode, text: string) {
    if (isText(node)) {
      node.text = text
    } else {
      throw new Error('node is not a VText (TODO: implement this by replacing object props so it\'s a VText?)')
    }

    node.renderer?.setNeedsRerender(node)
  }

  export function setProperty(node: VNode, name: string, value: any) {
    if (isElement(node) && VElement.hasProperty(node, name)) {
      node.props[name] = value
    } else {
      throw new Error(`property doesn't exist on ${VNode.type(node)} v-node: ${name}`)
    }

    node.renderer?.setNeedsRerender(node)
  }

  export function insertChild(parent: VNode, child: VNode, before?: VNode) {
    if (!isElement(parent)) {
      parent = convertIntoElement(parent)
    }

    if (child.parent !== null) {
      throw new Error('can\'t insert this child because it already has a parent')
    }
    child.parent = parent
    child.renderer = parent.renderer

    if (before === undefined) {
      parent.children.push(child)
    } else {
      const beforeIndex = parent.children.indexOf(before)
      if (beforeIndex === -1) {
        throw new Error('anchor node not found')
      }
      parent.children.splice(beforeIndex, 0, child)
    }

    parent.renderer?.setNeedsRerender({ parent, child, action: 'insert' })
  }

  export function removeChild(parent: VNode, child: VNode) {
    if (!isElement(parent)) {
      throw new Error('parent is not a VElement so it has no children, so you can\'t remove a child')
    }


    if (child.parent !== parent) {
      throw new Error('\'child\' to remove is not actually a child of this parent')
    }
    child.parent = null
    child.renderer = null

    const index = parent.children.indexOf(child)
    if (index === -1) {
      throw new Error('sanity check failed: child to remove not found')
    }
    parent.children.splice(index, 1)

    parent.renderer?.setNeedsRerender({ parent, child, action: 'remove' })
  }

  export function getParent(node: VNode): VElement {
    if (node.parent === null) {
      throw new Error('node has no parent')
    }
    return node.parent
  }

  export function getChildren(node: VNode): VNode[] {
    if (isElement(node)) {
      return node.children
    } else {
      return []
    }
  }

  export function getNextSibling(node: VNode): VNode | null {
    const parent = getParent(node)
    const indexInParent = parent.children.indexOf(node)
    if (indexInParent === -1) {
      throw new Error('sanity check failed: node\'s \'parent\' does not actually contain the node')
    }

    return parent.children[indexInParent + 1] ?? null
  }
}

export function VText(text: string): VText {
  return {
    text,
    parent: null,
    renderer: null
  }
}

export module VText {

}

export function VElement(tag: string): VElement {
  if (VElement.TAGS.has(tag as VTag)) {
    return {
      tag: tag as VTag,
      props: {},
      children: [],
      parent: null,
      renderer: null
    }
  } else {
    throw new Error(`not a v-tag: ${tag}`)
  }
}

export module VElement {
  export const TAGS: Set<VTag> = new Set(['box', 'image'])
  export const PROPERTIES: Record<VTag, Set<keyof VProps>> = {
    box: new Set([
      'className',
      'direction',
      'visible',
      'width',
      'height',
      'marginLeft',
      'marginRight',
      'marginTop',
      'marginBottom',
      'paddingLeft',
      'paddingRight',
      'paddingTop',
      'paddingBottom'
    ]),
    image: new Set([
      'className',
      'visible',
      'path',
      'width',
      'height'
    ])
  }

  export function hasProperty(elem: VElement, property: string): property is keyof VProps {
    return VElement.PROPERTIES[elem.tag].has(property as keyof VProps)
  }
}

export function VRoot(renderer: RendererImpl<any, any>): VElement {
  return {
    tag: 'box',
    props: {},
    children: [],
    parent: null,
    renderer
  }
}
