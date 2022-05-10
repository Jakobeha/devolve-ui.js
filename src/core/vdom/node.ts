import { BorderAttrs, BoxAttrs, ColorAttrs, PixiAttrs, SourceAttrs, TextAttrs } from 'core/vdom/attrs'
import type { DisplayObject } from 'pixi.js'
import { VComponent } from 'core/component'

export type VNode = VBox | VText | VColor | VBorder | VSource | VPixi<any>

interface VNodeCommon {
  // Really don't want to use both null and undefined
  parent?: VBox | 'none'
  component?: VComponent
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

export interface VBorder extends BorderAttrs, VNodeCommon {
  type: 'border'
}

export interface VSource extends SourceAttrs, VNodeCommon {
  type: 'source'
}

export interface VPixi<Pixi extends DisplayObject> extends PixiAttrs<Pixi>, VNodeCommon {
  type: 'pixi'
  // Not doing null | undefined
  pixi?: Pixi | 'terminal'
}

export module VNode {
  export function convertInto<T extends VNode> (target: Partial<VNode>, newData: T): asserts target is T {
    for (const prop in target) {
      if (prop !== 'parent' && prop !== 'pixi') {
        // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
        delete (target as any)[prop]
      }
    }
    for (const prop in newData) {
      if (prop === 'parent') {
        throw new Error('new data cannot have parent')
      } else if (prop === 'pixi') {
        throw new Error('new data cannot have pixi')
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

export function VBorder (attrs: BorderAttrs): VBorder {
  return { type: 'border', ...attrs }
}

export function VSource (attrs: SourceAttrs): VSource {
  return { type: 'source', ...attrs }
}

export function VPixi<Pixi extends DisplayObject> (attrs: PixiAttrs<Pixi>): VPixi<Pixi> {
  return { type: 'pixi', ...attrs }
}
