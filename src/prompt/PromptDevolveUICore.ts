import { Renderer, VNode } from 'core'
import type { TerminalRenderOptions } from 'renderer/cli'
import type { BrowserRenderOptions } from 'renderer/web'
import { VComponent } from 'core/component'
import { PromptArgs, PromptReplacedError, PromptReturn, PromptSpec, PromptTimeoutError } from 'prompt/prompt'
import { DevolveUICore } from 'core/DevolveUICore'
import { augmentSet } from 'core/augment-set'

export type RenderOptions =
  TerminalRenderOptions &
  BrowserRenderOptions

export interface PromptProps<PromptKeys extends string | number | symbol> {
  prompts: { [Key in PromptKeys]?: PromptSpec }
}

export abstract class PromptDevolveUICore<Props extends PromptProps<PromptKeys>, PromptKeys extends string | number | symbol> extends DevolveUICore<Props> {
  protected abstract mkRenderer (root: () => VNode, opts?: RenderOptions): Renderer

  /** Renders a HUD with the given content and doesn't clear, useful for logging */
  protected static _renderSnapshot<Props>(mkRenderer: (root: () => VNode, opts?: RenderOptions) => Renderer, RootComponent: (props: Props) => VNode, props: Props, opts?: RenderOptions): void {
    const renderer = mkRenderer(() => VComponent('RootComponent', props, RootComponent), opts)
    renderer.forceRerender()
    renderer.dispose()
  }

  constructor (RootComponent: (props: Props) => VNode, props: Omit<Props, keyof PromptProps<any>>, opts?: RenderOptions) {
    // We need to cast becuase this is slightly illegal: prompts should not require properties but we can't enforce that easily
    super(RootComponent, { ...props as Props, prompts: {} }, opts)
  }

  override setProps (newProps: Omit<Props, keyof PromptProps<any>>): void {
    for (const _key in newProps) {
      if (_key === 'prompts') {
        throw new Error('can\'t set prompts directly')
      }
    }
    super.setProps({ ...newProps as Props, prompts: this.props.prompts })
  }

  async prompt<Key extends PromptKeys>(key: Key, promptArgs: PromptArgs<Props['prompts'][Key]>, earlyCancelPing?: () => boolean): PromptReturn<Props['prompts'][Key]> {
    const oldPrompt = this.props.prompts[key]
    if (oldPrompt !== undefined) {
      // reject is a member of oldPrompt, even though it's not in the type, because we always set oldPromptand we include reject
      oldPrompt.reject!(new PromptReplacedError())
    }
    const earlyCancelPromise: PromptReturn<Props['prompts'][Key]> = new Promise((resolve, reject) => {
      setInterval(() => {
        if (earlyCancelPing?.() === true) {
          delete this.props.prompts[key]
          reject(new PromptTimeoutError())
        }
      }, 100)
    })
    // eslint-disable-next-line promise/param-names
    const promptPromise: PromptReturn<Props['prompts'][Key]> = new Promise((resolve_, reject_) => {
      if (key in this.props.prompts) {
        throw new Error('sanity check failed, probably a race condition')
      }

      // We want to delete the prompt before resolve completes, to prevent confusing race conditions
      const resolve = (arg: any): void => {
        delete this.props.prompts[key]
        resolve_(arg)
      }
      const reject = (arg: any): void => {
        delete this.props.prompts[key]
        reject_(arg)
      }
      this.props.prompts[key] = { ...promptArgs, resolve, reject }

      this.updateProps()
    })
    return await Promise.race([promptPromise, earlyCancelPromise])
  }

  protected override propsProxy<T extends object>(props: T): T {
    return augmentSet(props, path => {
      if (path === '.prompts') {
        throw new Error('can\'t set prompts')
      }
      this.updateProps()
    })
  }
}
