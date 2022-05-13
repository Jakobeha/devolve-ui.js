import { BorderAttrs, BoxAttrs, ColorAttrs, PixiAttrs, SourceAttrs, TextAttrs } from 'core/view/attrs'
import type { DisplayObject } from 'pixi.js'
import { VNode } from 'core'

export type VView = VBox | VText | VColor | VBorder | VSource | VPixi<any>

interface VViewCommon {
  readonly id: number
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
  return { id: VNode.nextId(), type: 'text', text, ...attrs }
}

export function VBox (children: VNode[], attrs: BoxAttrs): VBox {
  return { id: VNode.nextId(), type: 'box', children, ...attrs }
}

export function VColor (attrs: ColorAttrs): VColor {
  return { id: VNode.nextId(), type: 'color', ...attrs }
}

export function VBorder (attrs: BorderAttrs): VBorder {
  return { id: VNode.nextId(), type: 'border', ...attrs }
}

export function VSource (attrs: SourceAttrs): VSource {
  return { id: VNode.nextId(), type: 'source', ...attrs }
}

export function VPixi<Pixi extends DisplayObject> (attrs: PixiAttrs<Pixi>): VPixi<Pixi> {
  return { id: VNode.nextId(), type: 'pixi', ...attrs, pixi: null }
}
