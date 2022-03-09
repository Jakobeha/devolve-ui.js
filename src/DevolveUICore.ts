import { Renderer, VNode } from 'core'
import type { TerminalRenderOptions } from 'renderer/cli'
import type { BrowserRenderOptions } from 'renderer/web'
import { VComponent } from 'core/component'
import { PromptArgs, PromptReplacedError, PromptReturn, PromptSpec, PromptTimeoutError } from 'prompt'
import { DeepReadonly } from '@raycenity/misc-ts'

export type RenderOptions =
  TerminalRenderOptions &
  BrowserRenderOptions

export interface RootProps<PromptKeys extends string | number | symbol> {
  prompts: { [Key in PromptKeys]?: PromptSpec }
}

export abstract class DevolveUICore<Props extends RootProps<PromptKeys>, PromptKeys extends string | number | symbol> {
  protected abstract mkRenderer (root: () => VNode, opts?: RenderOptions): Renderer

  private readonly instance: Renderer
  private readonly props: Props
  /** A proxy which sets the given property */
  readonly p: Omit<Props, keyof RootProps<any>>

  /** Renders a HUD with the given content and doesn't clear, useful for logging */
  protected static _renderSnapshot<Props>(mkRenderer: (root: () => VNode, opts?: RenderOptions) => Renderer, RootComponent: (props: Props) => VNode, props: Props, opts?: RenderOptions): void {
    const renderer = mkRenderer(() => VComponent('RootComponent', props, RootComponent), opts)
    renderer.forceRerender()
    renderer.dispose()
  }

  constructor (private readonly RootComponent: (props: Props) => VNode, props: Omit<Props, keyof RootProps<any>>, opts?: RenderOptions) {
    // Idk why the cast is necessary
    this.props = {
      ...props as Props,
      prompts: {}
    }
    this.instance = this.mkRenderer(() => VComponent('RootComponent', this.props, RootComponent), opts)
    this.p = this.propsProxy(this.props, true)
  }

  getProps (): DeepReadonly<Props> {
    return this.props
  }

  setProps (newProps: Omit<Props, keyof RootProps<any>>): void {
    for (const _key in newProps) {
      if (_key === 'prompts') {
        throw new Error('can\'t set prompts directly')
      }
      const key: Exclude<keyof Props, keyof RootProps<any>> = _key as any
      this.props[key] = newProps[key]
    }
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

  show (): void {
    this.instance.show()
  }

  hide (): void {
    this.instance.hide()
  }

  close (): void {
    this.instance.dispose()
  }

  private propsProxy<T extends object>(props: T, isRoot: boolean = false): T {
    return new Proxy(props, {
      get: (target: T, p: string | symbol): any => {
        const value = (target as any)[p]
        if (typeof value === 'object' || typeof value === 'function') {
          return this.propsProxy(value)
        } else {
          return value
        }
      },
      set: (target: T, p: string | symbol, value: any): boolean => {
        if (isRoot && (p === 'prompts')) {
          throw new Error('can\'t set prompts')
        }
        (target as any)[p] = value
        this.updateProps()
        return true
      },
      apply: (target: T, thisArg: any, args: any[]): any => {
        // Function might change stuff, so we reroot (e.g. in arrays)
        // Worst case scenario we just reroot when not necessary
        this.updateProps()
        return Reflect.apply(target as Function, thisArg, args)
      }
    })
  }

  private updateProps (): void {
    // reroot only rerenders once per frame, so batch update functionality isn't needed
    this.instance.reroot(this.props)
  }
}
