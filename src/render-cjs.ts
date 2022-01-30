import type { TerminalRenderOptions } from 'renderer/cli'
import type { BrowserRenderOptions } from 'renderer/web'
import type { RendererImpl } from 'renderer/common'
import { PLATFORM, Renderer, VNode } from 'core'

type RenderOptions =
  TerminalRenderOptions &
  BrowserRenderOptions

function throw_ (error: Error): never {
  throw error
}

let PlatformRendererImpl: new (root: () => VNode, opts?: RenderOptions) => RendererImpl<any, any>
/* eslint-disable no-useless-catch */
/* eslint-disable @typescript-eslint/restrict-template-expressions */
/* eslint-disable @typescript-eslint/no-var-requires */
try {
  PlatformRendererImpl =
    PLATFORM === 'web'
      ? require('renderer/web').BrowserRendererImpl
      : PLATFORM === 'cli'
        ? require('renderer/cli').TerminalRendererImpl
        : throw_(new Error(`Unsupported platform: ${PLATFORM}`))
} catch (error) {
  // Try block is needed to suppress esbuild warning
  throw error
}
/* eslint-enable no-useless-catch */
/* eslint-enable @typescript-eslint/restrict-template-expressions */
/* eslint-enable @typescript-eslint/no-var-requires */

export function render (root: () => VNode, opts?: RenderOptions): Renderer {
  const renderer = new PlatformRendererImpl(root, opts)
  renderer.start()
  return renderer
}
