import { VElement, VNode, VRoot } from 'universal/vdom'

type Timer = NodeJS.Timer

export interface Renderer {
  start: (fps?: number) => void
  stop: () => void
  setNeedsRerender: () => void
  dispose: () => void
}

export interface CoreRenderOptions {
  fps?: number
}

export type RenderDiff = VNode | {
  parent: VNode
  child: VNode
  action: 'insert' | 'remove'
}

export abstract class CoreAssetCacher {
  private assets: { [key: string]: any } = {}

  protected get<T>(path: string, construct: (path: string) => T): T {
    if (this.assets.has(path)) {
      return this.assets.get(path)
    } else {
      const image = construct(path)
      this.assets.set(path, image)
      return image
    }
  }

  protected getAsync<T>(path: string, construct: (path: string) => Promise<T>): [T | null, (didFind: () => void) => void] {
    if (this.assets.has(path)) {
      return [this.assets.get(path), () => {}]
    } else {
      this.assets.set(path, null)
      return [null, (didFind: () => void) => {
        construct(path).then(image => {
          this.assets.set(path, image)
          didFind()
        })
      }]
    }
  }
}

export abstract class RendererImpl<VRender, VAssetCacher extends CoreAssetCacher> implements Renderer {
  static readonly DEFAULT_FPS: number = 20

  private readonly defaultFps: number
  private readonly _root: VElement = VRoot(this)
  protected readonly assets: VAssetCacher

  private needsRerender: boolean = true
  private timer: Timer | null = null

  get root(): VElement {
    return this._root
  }

  constructor(assetCacher: VAssetCacher, {fps}: CoreRenderOptions) {
    this.defaultFps = fps ?? RendererImpl.DEFAULT_FPS
    this.assets = assetCacher
  }

  start(fps?: number) {
    if (this.timer !== null) {
      throw new Error('Renderer is already running')
    }

    this.timer = setInterval(() => {
      if (this.needsRerender) {
        this.rerender()
      }
    }, 1 / (fps ?? this.defaultFps))
  }

  stop() {
    if (this.timer === null) {
      throw new Error('Renderer is not running')
    }

    clearInterval(this.timer)
    this.timer = null
  }

  setNeedsRerender(_diff?: RenderDiff) {
    // TODO: Cache if the diff contains an important node
    this.needsRerender = true
  }

  private rerender() {
    if (!this.needsRerender) return

    this.needsRerender = false
    this.clear()
    this.render()
  }

  protected abstract clear(): void

  private render() {
    this.writeRender(this.renderNode(this.root))
  }

  protected abstract writeRender(render: VRender): void

  protected abstract renderNode(node: VNode): VRender

  dispose() {
    if (this.timer !== null) {
      this.stop()
    }
  }
}
