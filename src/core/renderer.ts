import { VView } from 'core/view/view'
import { Size } from 'core/view'

export interface Renderer {
  forceRerender: () => void
  reroot: <Props>(props?: Props, root?: (props: Props) => VView) => void
  show: () => void
  hide: () => void
  dispose: () => void
}

export interface CoreRenderOptions {
  fps?: number
}

export const DEFAULT_CORE_RENDER_OPTIONS: Required<CoreRenderOptions> = {
  fps: 20
}

export const DEFAULT_COLUMN_SIZE: Size = {
  width: 7,
  height: 14
}
