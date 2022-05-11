import { BorderAttrs, BoxAttrs, ColorAttrs, PixiAttrs, SourceAttrs, TextAttrs } from 'core/vdom/attrs'
import type { DisplayObject } from 'pixi.js'
import { VComponent } from 'core/component'
import { assert } from '@raycenity/misc-ts'

// TODO: Rename VNode to VView
export type VNodeNode = VNode | VComponent

export module VNodeNode {
  export function update (node: VNodeNode, updatePath: string): void {
    updatePath += `/${node.key ?? ''}`
    if (node.type === 'component') {
      VComponent.update(node, updatePath)
    } else if (node.type === 'box') {
      node.children.forEach((child, index) => {
        const updateSubpath = `${updatePath}[${index}]`
        update(child, updateSubpath)
      })
    }
  }

  export function view (node: VNodeNode): VNode {
    if (node.type === 'component') {
      assert(node.node !== null, `tried to get view from uninitialized component: ${node.key}. It should've been initialized earlier`)
      return node.node
    } else {
      return node
    }
  }
}

export type VNode = VBox | VText | VColor | VBorder | VSource | VPixi<any>

interface VNodeCommon {
  readonly type: string
}

export interface VBox extends BoxAttrs, VNodeCommon {
  readonly type: 'box'
  readonly children: readonly VNodeNode[]
}

export interface VText extends TextAttrs, VNodeCommon {
  readonly type: 'text'
  readonly text: string
}

export interface VColor extends ColorAttrs, VNodeCommon {
  readonly type: 'color'
}

export interface VBorder extends BorderAttrs, VNodeCommon {
  readonly type: 'border'
}

export interface VSource extends SourceAttrs, VNodeCommon {
  readonly type: 'source'
}

export interface VPixi<Pixi extends DisplayObject> extends PixiAttrs<Pixi>, VNodeCommon {
  readonly type: 'pixi'
  // Not doing null | undefined
  pixi: Pixi | 'terminal' | null
}

export function VText (text: string, attrs: TextAttrs): VText {
  return { type: 'text', text, ...attrs }
}

export function VBox (children: VNodeNode[], attrs: BoxAttrs): VBox {
  return { type: 'box', children, ...attrs }
}

export function VColor (attrs: ColorAttrs): VColor {
  return { type: 'color', ...attrs }
}

export function VBorder (attrs: BorderAttrs): VBorder {
  return { type: 'border', ...attrs }
}

export function VSource (attrs: SourceAttrs): VSource {
  return { type: 'source', ...attrs }
}

export function VPixi<Pixi extends DisplayObject> (attrs: PixiAttrs<Pixi>): VPixi<Pixi> {
  return { type: 'pixi', ...attrs, pixi: null }
}
