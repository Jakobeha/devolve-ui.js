import { Lens } from 'core/lens'
import { VComponent } from 'core/component'
import { Renderer, VView } from 'core/index'
import type { TerminalRenderOptions } from 'renderer/cli'
import type { BrowserRenderOptions } from 'renderer/web'
import { DeepReadonly } from '@raycenity/misc-ts'

export type RenderOptions =
  TerminalRenderOptions &
  BrowserRenderOptions

export abstract class DevolveUICore<Props extends object> {
  protected abstract mkRenderer (root: () => VComponent, opts?: RenderOptions): Renderer

  private readonly instance: Renderer
  protected readonly props: Props
  /** A proxy which sets the given property */
  readonly p: Lens<Props>

  /** Renders a HUD with the given content and doesn't clear, useful for logging */
  protected static _renderSnapshot<Props>(mkRenderer: (root: () => VComponent, opts?: RenderOptions) => Renderer, RootComponent: (props: Props) => VView, props: Props, opts?: RenderOptions): void {
    const renderer = mkRenderer(() => VComponent('RootComponent', props, RootComponent), opts)
    renderer.forceRerender()
    renderer.dispose()
  }

  constructor (private readonly RootComponent: (props: Props) => VView, props: Props, opts?: RenderOptions) {
    // Idk why the cast is necessary
    this.props = { ...props }
    this.instance = this.mkRenderer(() => VComponent('RootComponent', this.props, RootComponent), opts)
    this.p = this.propsLens(this.props)
  }

  getProps (): DeepReadonly<Props> {
    return this.props as DeepReadonly<Props>
  }

  setProps (newProps: Props): void {
    Object.assign(this.props, newProps)
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

  protected propsLens<T extends object>(props: T): Lens<T> {
    const lens = Lens(props)
    Lens.onSet(lens, () => {
      this.updateProps()
    })
    return lens
  }

  protected updateProps (): void {
    // reroot only rerenders once per frame, so batch update functionality isn't needed
    this.instance.reroot(this.props)
  }
}
