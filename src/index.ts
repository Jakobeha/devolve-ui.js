import {
  BoxAttrs,
  elements as cliElements,
  PrimitiveAttrs,
  render as renderCli,
  renderer as cliRenderer,
  VNode
} from 'cli'
import { elements as webElements, JSX, render as renderWeb, renderer as webRenderer } from 'web'
import { Elements, Renderer } from 'universal'

export * from 'universal'

export type Platform = 'cli' | 'browser'

declare const NODE_OPAQUE_HACK: unique symbol
export type PlatformNode = {
  opaque: typeof NODE_OPAQUE_HACK
}

export const platform: Platform = typeof window === 'undefined' ? 'cli' : 'browser'

export const renderer: Renderer<PlatformNode> =
  (platform === 'cli' ? cliRenderer : webRenderer) as unknown as Renderer<PlatformNode>

export const elements: Elements<PlatformNode> =
  (platform === 'cli' ? cliElements : webElements) as unknown as Elements<PlatformNode>

export const {
  effect,
  memo,
  createComponent,
  insert,
  spread,
  mergeProps
} = renderer

export const {
  Text,
  Box,
  Newline,
  For,
  Index,
  Show,
  Switch,
  Match
} = elements

export function render(node: () => PlatformNode) {
  if (platform === 'cli') {
    return renderCli(node as unknown as () => VNode)
  } else {
    return renderWeb(node as unknown as () => JSX.Element, document.body)
  }
}
