import { VNode } from 'core/vdom'

export interface Renderer {
  reroot: (root: () => VNode) => void
  start: (fps?: number) => void
  stop: () => void
  dispose: () => void
}

export interface CoreRenderOptions {
  fps?: number
}
