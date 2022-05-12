// noinspection DuplicatedCode

import { DevolveUICore, RenderOptions } from 'core/DevolveUICore'
import { PromptDevolveUICore, PromptProps } from 'prompt/PromptDevolveUICore'
import type { RendererImpl } from 'renderer/common'
import { PLATFORM, Renderer, VComponent, VNode } from 'core'

let PlatformRendererImpl: new (root: () => VComponent, opts?: RenderOptions) => RendererImpl<any, any>
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

export type { RenderOptions, PromptProps }
export * from 'prompt/prompt'

export class DevolveUI<Props extends object> extends DevolveUICore<Props> {
  protected override mkRenderer (root: () => VComponent, opts?: RenderOptions): Renderer {
    return new PlatformRendererImpl(root, opts)
  }

  static renderSnapshot<Props> (RootComponent: (props: Props) => VNode, props: Props, opts?: RenderOptions): void {
    return DevolveUICore._renderSnapshot((root, opts) => new PlatformRendererImpl(root, opts), RootComponent, props, opts)
  }
}

export class PromptDevolveUI<
  Props extends PromptProps<PromptKeys>,
  PromptKeys extends string | number | symbol = keyof Props['prompts']
> extends PromptDevolveUICore<Props, PromptKeys> {
  protected override mkRenderer (root: () => VComponent, opts?: RenderOptions): Renderer {
    return new PlatformRendererImpl(root, opts)
  }

  static renderSnapshot<Props> (RootComponent: (props: Props) => VNode, props: Props, opts?: RenderOptions): void {
    return DevolveUICore._renderSnapshot((root, opts) => new PlatformRendererImpl(root, opts), RootComponent, props, opts)
  }
}
