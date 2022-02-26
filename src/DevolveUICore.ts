import { Renderer, VNode } from 'core'
import type { TerminalRenderOptions } from 'renderer/cli'
import type { BrowserRenderOptions } from 'renderer/web'
import { VComponent } from 'core/component'
import { PromptArgs, PromptReplacedError, PromptReturn, PromptSpec, PromptTimeoutError } from 'flash-prompt'

export type RenderOptions =
  TerminalRenderOptions &
  BrowserRenderOptions

export interface RootProps<MessageKeys extends string | number | symbol, PromptKeys extends string | number | symbol> {
  messages: { [Key in MessageKeys]?: any }
  prompts: { [Key in PromptKeys]?: PromptSpec }
}

export abstract class DevolveUICore<Props extends RootProps<MessageKeys, PromptKeys>, MessageKeys extends string | number | symbol, PromptKeys extends string | number | symbol> {
  protected abstract mkRenderer (root: () => VNode, opts?: RenderOptions): Renderer

  private readonly instance: Renderer
  private readonly props: Props

  /** Renders a HUD with the given content and doesn't clear, useful for logging */
  protected static _renderSnapshot<Props> (mkRenderer: (root: () => VNode, opts?: RenderOptions) => Renderer, RootComponent: (props: Props) => VNode, props: Props, opts?: RenderOptions): void {
    const renderer = mkRenderer(() => VComponent('RootComponent', () => RootComponent(props)), opts)
    renderer.forceRerender()
    renderer.dispose()
  }

  constructor (private readonly RootComponent: (props: Props) => VNode, staticProps: Omit<Props, keyof RootProps<any, any>>, opts?: RenderOptions) {
    // Idk why the cast is necessary
    this.props = {
      ...staticProps as Props,
      messages: {},
      prompts: {}
    }
    this.instance = this.mkRenderer(() => VComponent('RootComponent', () => RootComponent(this.props)), opts)
  }

  setMessage<Key extends MessageKeys> (key: Key, message: Props['messages'][Key]): void {
    this.props.messages[key] = message
    this.instance.reroot()
  }

  clearMessage<Key extends MessageKeys> (key: Key): void {
    delete this.props.messages[key]
    this.instance.reroot()
  }

  clearMessages (): void {
    this.props.messages = {}
    this.instance.reroot()
  }

  async prompt<Key extends PromptKeys>(key: Key, promptArgs: PromptArgs<Props['prompts'][Key]>, earlyCancelPing?: () => boolean): PromptReturn<Props['prompts'][Key]> {
    const oldPrompt = this.props.prompts[key]
    if (oldPrompt !== undefined) {
      oldPrompt.reject(new PromptReplacedError())
    }
    const earlyCancelPromise: PromptReturn<Props['prompts'][Key]> = new Promise((resolve, reject) => {
      setInterval(() => {
        if (earlyCancelPing?.() === true) {
          reject(new PromptTimeoutError())
        }
      }, 100)
    })
    const promptPromise: PromptReturn<Props['prompts'][Key]> = new Promise((resolve, reject) => {
      if (key in this.props.prompts) {
        throw new Error('sanity check failed, probably a race condition')
      }
      this.props.prompts[key] = { ...promptArgs, resolve, reject }
      this.instance.reroot()
    })
    return await Promise.race([promptPromise, earlyCancelPromise]).finally(() => {
      delete this.props.prompts[key]
    })
  }

  show (): void {
    this.instance.show()
  }

  hide (): void {
    this.instance.hide()
  }

  close (): void {
    this.instance.dispose()
  }
}
