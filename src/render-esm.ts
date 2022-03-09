import { DevolveUICore, RenderOptions, RootProps } from 'DevolveUICore'
import type { RendererImpl } from 'renderer/common'
import { PLATFORM, Renderer, VNode } from 'core'

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

export type { RenderOptions, RootProps }
export * from 'prompt'

export class DevolveUI<
  Props extends RootProps<PromptKeys>,
  PromptKeys extends string | number | symbol = keyof Props['prompts']
> extends DevolveUICore<Props, PromptKeys> {
  protected override mkRenderer (root: () => VNode, opts?: RenderOptions): Renderer {
    return new PlatformRendererImpl(root, opts)
  }

  static renderSnapshot<Props> (RootComponent: (props: Props) => VNode, props: Props, opts?: RenderOptions): void {
    return DevolveUICore._renderSnapshot((root, opts) => new PlatformRendererImpl(root, opts), RootComponent, props, opts)
  }
}
