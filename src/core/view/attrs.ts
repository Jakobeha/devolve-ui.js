import { BoundingBox, Bounds, BoundsSpec, Size } from 'core/view/bounds'
import { BorderStyle } from 'core/view/border-style'
import { Color, ColorSpec, LCHColor, RGBColor } from 'core/view/color'
import type { DisplayObject } from 'pixi.js'
import { DelayedSubLayout } from 'core'
import { CustomDelayedSubLayout } from 'core/view/sub-layout'

export interface CommonAttrs {
  readonly bounds?: Bounds
  readonly visible?: boolean
  readonly key?: string
}

export interface BoxAttrs extends CommonAttrs {
  readonly sublayout?: DelayedSubLayout
  readonly clip?: boolean
  readonly extend?: boolean
}

export interface TextAttrs extends CommonAttrs {
  readonly color: Color | null
  readonly wrapMode?: 'word' | 'char' | 'clip'
}

export interface ColorAttrs extends CommonAttrs {
  readonly color: Color
}

export interface BorderAttrs extends CommonAttrs {
  readonly color: Color | null
  readonly style: BorderStyle
}

export interface SourceAttrs extends CommonAttrs {
  readonly src: string
}

export interface PixiAttrs<Pixi extends DisplayObject> extends CommonAttrs {
  /**
   * Returns the pixi {@link DisplayObject}'s "size" in order to layout other nodes,
   * *and* allows you to update the {@link DisplayObject}'s size from the bounding box,
   * which includes inferred size (you can return a different size), and column size.
   */
  readonly getSize?: (pixi: Pixi, bounds: BoundingBox, columnSize: Size) => Size
}

export interface JSXSubLayoutAttrs {
  storeBoundsIn?: string
  keepBounds?: string | string[]
  customSublayout?: CustomDelayedSubLayout
}

export type JSXTextAttrs = JSXColorAttrs<TextAttrs>
export type JSXBoxAttrs = Omit<BoxAttrs, 'sublayout'> & Omit<DelayedSubLayout, 'store' | 'keep' | 'custom'> & JSXSubLayoutAttrs & BoundsSpec
export type JSXColorAttrs<T extends CommonAttrs & { color: Color | null } = ColorAttrs> = Omit<T, 'color'> & Partial<{ color: ColorSpec } & LCHColor & RGBColor> & BoundsSpec
export type JSXBorderAttrs = JSXColorAttrs<BorderAttrs>
export type JSXSourceAttrs = SourceAttrs & BoundsSpec
export type JSXPixiAttrs<Pixi extends DisplayObject> = PixiAttrs<Pixi> & BoundsSpec
