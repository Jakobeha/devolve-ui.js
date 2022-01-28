import { VNode } from 'universal'
import { Renderer } from 'universal/renderer'
import { Platform, PLATFORM } from 'universal/platform'
import { TerminalRendererImpl, TerminalRenderOptions } from 'cli/renderer'
import { BrowserRendererImpl, BrowserRenderOptions } from 'web/renderer'

export * from 'types'
export * from 'universal'

type RenderOptions =
  TerminalRenderOptions &
  BrowserRenderOptions &
  { platform?: Platform }

export function render(template: () => VNode, opts?: RenderOptions): Renderer {
  const platform = opts?.platform ?? PLATFORM
  const renderer =
    platform === 'web' ? new BrowserRendererImpl(opts) :
      platform === 'cli' ? new TerminalRendererImpl(opts) :
        undefined
  if (renderer === undefined) {
    throw new Error(`Unsupported platform: ${platform}`)
  }
  renderer.start()
  return renderer
}
