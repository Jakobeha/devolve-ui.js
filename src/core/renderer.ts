import { VNode } from 'core/vdom/node'

export interface Renderer {
  reroot: (root: () => VNode) => void
  show: () => void
  hide: () => void
  dispose: () => void
}

export interface CoreRenderOptions {
  fps?: number
}
