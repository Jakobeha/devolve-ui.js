import { TerminalRendererImpl, TerminalRenderOptions } from 'renderer/cli'
import { BrowserRendererImpl, BrowserRenderOptions } from 'renderer/web'
import { PLATFORM, Platform, Renderer, VNode } from 'core'

type RenderOptions =
  TerminalRenderOptions &
  BrowserRenderOptions &
  { platform?: Platform }

export function render (root: () => VNode, opts?: RenderOptions): Renderer {
  const platform = opts?.platform ?? PLATFORM
  const renderer =
    platform === 'web'
      ? new BrowserRendererImpl(root, opts)
      : platform === 'cli'
        ? new TerminalRendererImpl(root, opts)
        : undefined
  if (renderer === undefined) {
    throw new Error(`Unsupported platform: ${platform}`)
  }
  renderer.start()
  return renderer
}
