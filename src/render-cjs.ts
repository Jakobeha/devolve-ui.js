import { DevolveUICore, RenderOptions, RootProps } from 'DevolveUICore'
import type { RendererImpl } from 'renderer/common'
import { PLATFORM, Renderer, VNode } from 'core'

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

export type { RenderOptions, RootProps }
export * from 'prompt'

export class DevolveUI<
  Props extends RootProps<MessageKeys, PromptKeys>,
  MessageKeys extends string | number | symbol = keyof Props['messages'],
  PromptKeys extends string | number | symbol = keyof Props['prompts']
> extends DevolveUICore<Props, MessageKeys, PromptKeys> {
  protected override mkRenderer (root: () => VNode, opts?: RenderOptions): Renderer {
    return new PlatformRendererImpl(root, opts)
  }

  static renderSnapshot<Props> (RootComponent: (props: Props) => VNode, props: Props, opts?: RenderOptions): void {
    return DevolveUICore._renderSnapshot((root, opts) => new PlatformRendererImpl(root, opts), RootComponent, props, opts)
  }
}
