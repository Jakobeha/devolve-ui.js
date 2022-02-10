import type { TerminalRenderOptions } from 'renderer/cli'
import type { BrowserRenderOptions } from 'renderer/web'
import type { RendererImpl } from 'renderer/common'
import { PLATFORM, Renderer, VNode } from 'core'

type RenderOptions =
  TerminalRenderOptions &
  BrowserRenderOptions

/* eslint-disable @typescript-eslint/restrict-template-expressions */
const PlatformRendererImpl: new (root: () => VNode, opts?: RenderOptions) => RendererImpl<any, any> = await (
  PLATFORM === 'web'
    ? import('renderer/web').then(module => module.BrowserRendererImpl)
    : PLATFORM === 'cli'
      ? Promise.all([import('renderer/cli'), import('readline')]).then(([module, readline]) => {
        module.initModule({ readline })
        return module.TerminalRendererImpl
      })
      : Promise.reject(new Error(`Unsupported platform: ${PLATFORM}`))
)
/* eslint-enable @typescript-eslint/restrict-template-expressions */

export function render (root: () => VNode, opts?: RenderOptions): Renderer {
  const renderer = new PlatformRendererImpl(root, opts)
  renderer.start()
  return renderer
}
