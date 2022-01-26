import { render as renderCli, renderer as cliRenderer, VNode } from 'cli'
import { JSX, render as renderWeb, renderer as webRenderer } from 'web'
import { Renderer } from 'universal'

export * from 'universal'

type Platform = 'cli' | 'browser'

declare const NODE_OPAQUE_HACK: unique symbol
type PlatformNode = typeof NODE_OPAQUE_HACK

export const platform: Platform = typeof window === 'undefined' ? 'cli' : 'browser'

export const renderer: Renderer<PlatformNode> =
  (platform === 'cli' ? cliRenderer : webRenderer) as unknown as Renderer<PlatformNode>

export const {
  effect,
  memo,
  createComponent,
  insert,
  spread,
  mergeProps
} = renderer

export function render(node: () => PlatformNode) {
  if (platform === 'cli') {
    return renderCli(node as unknown as () => VNode)
  } else {
    return renderWeb(node as unknown as () => JSX.Element, document.body)
  }
}
