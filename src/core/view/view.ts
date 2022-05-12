import { BorderAttrs, BoxAttrs, ColorAttrs, PixiAttrs, SourceAttrs, TextAttrs } from 'core/view/attrs'
import type { DisplayObject } from 'pixi.js'
import { VComponent } from 'core/component'
import { assert } from '@raycenity/misc-ts'

export type VNode = VView | VComponent

export module VNode {
  export function update (node: VNode, updatePath: string): void {
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

  export function view (node: VNode): VView {
    if (node.type === 'component') {
      assert(node.view !== null, `tried to get view from uninitialized component: ${node.key}. It should've been initialized earlier`)
      return node.view
    } else {
      return node
    }
  }
}

export type VView = VBox | VText | VColor | VBorder | VSource | VPixi<any>

interface VViewCommon {
  readonly type: string
}

export interface VBox extends BoxAttrs, VViewCommon {
  readonly type: 'box'
  readonly children: readonly VNode[]
}

export interface VText extends TextAttrs, VViewCommon {
  readonly type: 'text'
  readonly text: string
}

export interface VColor extends ColorAttrs, VViewCommon {
  readonly type: 'color'
}

export interface VBorder extends BorderAttrs, VViewCommon {
  readonly type: 'border'
}

export interface VSource extends SourceAttrs, VViewCommon {
  readonly type: 'source'
}

export interface VPixi<Pixi extends DisplayObject> extends PixiAttrs<Pixi>, VViewCommon {
  readonly type: 'pixi'
  // Not doing null | undefined
  pixi: Pixi | 'terminal' | null
}

export function VText (text: string, attrs: TextAttrs): VText {
  return { type: 'text', text, ...attrs }
}

export function VBox (children: VNode[], attrs: BoxAttrs): VBox {
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
