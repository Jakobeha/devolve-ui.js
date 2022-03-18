import { BoundingBox, Bounds, BoundsSpec, Size, SubLayout } from 'core/vdom/bounds'
import { BorderStyle } from 'core/vdom/border-style'
import { Color, ColorSpec, LCHColor, RGBColor } from 'core/vdom/color'
import type { DisplayObject } from 'pixi.js'

export interface CommonAttrs {
  bounds?: Bounds
  visible?: boolean
  key?: string
}

export interface BoxAttrs extends CommonAttrs {
  sublayout?: SubLayout
}

export interface TextAttrs extends CommonAttrs {
  color: Color | null
  wrapMode?: 'word' | 'char' | 'clip'
}

export interface ColorAttrs extends CommonAttrs {
  color: Color
}

export interface BorderAttrs extends CommonAttrs {
  color: Color | null
  style: BorderStyle
}

export interface SourceAttrs extends CommonAttrs {
  src: string
}

export interface PixiAttrs<Pixi extends DisplayObject> extends CommonAttrs {
  /**
   * Returns the pixi {@link DisplayObject}'s "size" in order to layout other nodes,
   * *and* allows you to update the {@link DisplayObject}'s size from the bounding box,
   * which includes inferred size (you can return a different size), and column size.
   */
  getSize?: (pixi: Pixi, bounds: BoundingBox, columnSize: Size) => Size
}

export type JSXTextAttrs = JSXColorAttrs<TextAttrs>
export type JSXBoxAttrs = Omit<BoxAttrs, 'sublayout'> & SubLayout & BoundsSpec
export type JSXColorAttrs<T extends CommonAttrs & { color: Color | null } = ColorAttrs> = Omit<T, 'color'> & Partial<{ color: ColorSpec } & LCHColor & RGBColor> & BoundsSpec
export type JSXBorderAttrs = JSXColorAttrs<BorderAttrs>
export type JSXSourceAttrs = SourceAttrs & BoundsSpec
export type JSXPixiAttrs<Pixi extends DisplayObject> = PixiAttrs<Pixi> & BoundsSpec
