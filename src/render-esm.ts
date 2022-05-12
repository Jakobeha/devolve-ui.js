// noinspection DuplicatedCode

import { DevolveUICore, RenderOptions } from 'core/DevolveUICore'
import { PromptDevolveUICore, PromptProps } from 'prompt/PromptDevolveUICore'
import type { RendererImpl } from 'renderer/common'
import { PLATFORM, Renderer, VComponent, VView } from 'core'

/* eslint-disable @typescript-eslint/restrict-template-expressions */
const PlatformRendererImpl: new (root: () => VComponent, opts?: RenderOptions) => RendererImpl<any, any> = await (
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

export type { RenderOptions, PromptProps }
export * from 'prompt/prompt'

export class DevolveUI<Props extends object> extends DevolveUICore<Props> {
  protected override mkRenderer (root: () => VComponent, opts?: RenderOptions): Renderer {
    return new PlatformRendererImpl(root, opts)
  }

  static renderSnapshot<Props> (RootComponent: (props: Props) => VView, props: Props, opts?: RenderOptions): void {
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

  static renderSnapshot<Props> (RootComponent: (props: Props) => VView, props: Props, opts?: RenderOptions): void {
    return DevolveUICore._renderSnapshot((root, opts) => new PlatformRendererImpl(root, opts), RootComponent, props, opts)
  }
}
