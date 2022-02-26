import { VNode } from 'core/vdom/node'

export interface Renderer {
  forceRerender: () => void
  reroot: (root?: () => VNode) => void
  show: () => void
  hide: () => void
  dispose: () => void
}

export interface CoreRenderOptions {
  fps?: number
}
