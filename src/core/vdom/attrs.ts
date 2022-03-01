import { Bounds, BoundsSpec, SubLayout } from 'core/vdom/bounds'
import { Color, ColorName, LCHColor, RGBColor } from 'core/vdom/color'

export interface CommonAttrs {
  bounds?: Bounds
  visible?: boolean
  key?: string
}

export interface BoxAttrs extends CommonAttrs {
  sublayout?: SubLayout
}

export interface TextAttrs extends CommonAttrs {
  wrapMode?: 'word' | 'char' | 'clip'
}

export interface ColorAttrs extends CommonAttrs {
  color: Color
}

export interface SourceAttrs extends CommonAttrs {
  src: string
}

export type JSXTextAttrs = TextAttrs & BoundsSpec
export type JSXBoxAttrs = Omit<BoxAttrs, 'sublayout'> & SubLayout & BoundsSpec
export type JSXColorAttrs = Omit<ColorAttrs, 'color'> & Partial<{ color: Color } & LCHColor & RGBColor & { name: ColorName }> & BoundsSpec
export type JSXSourceAttrs = SourceAttrs & BoundsSpec
