import { Bounds, SubLayout } from 'core/vdom/bounds'
import { LCHColor } from 'core/vdom/color'

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
  color: LCHColor
}

export interface SourceAttrs extends CommonAttrs {
  src: string
}

export type JSXBoxAttrs = Omit<BoxAttrs, 'sublayout'> & SubLayout
