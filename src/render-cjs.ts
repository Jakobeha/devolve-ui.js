import type { TerminalRenderOptions } from 'renderer/cli'
import type { BrowserRenderOptions } from 'renderer/web'
import type { RendererImpl } from 'renderer/common'
import { PLATFORM, Renderer, VNode } from 'core'

type RenderOptions =
  TerminalRenderOptions &
  BrowserRenderOptions

let PlatformRendererImpl: new (root: () => VNode, opts?: RenderOptions) => RendererImpl<any, any>
/* eslint-disable no-useless-catch */
/* eslint-disable @typescript-eslint/restrict-template-expressions */
/* eslint-disable @typescript-eslint/no-var-requires */
try {
  if (PLATFORM === 'web') {
    PlatformRendererImpl = require('renderer/web').BrowserRendererImpl
  } else if (PLATFORM === 'cli') {
    const cliModule = require('renderer/cli')
    cliModule.initModule({ readline: require('readline') })
    PlatformRendererImpl = cliModule.TerminalRendererImpl
  } else {
    // noinspection ExceptionCaughtLocallyJS
    throw new Error(`Unsupported platform: ${PLATFORM}`)
  }
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
