import { VNode } from 'core/vdom'
import { CoreRenderOptions, Renderer } from 'core/renderer'
import { VRoot } from 'core/component'

type Timer = NodeJS.Timer

export type RenderDiff = VNode

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
  private root: VNode
  protected readonly assets: VAssetCacher

  private readonly cachedRenders: Map<VNode, VRender> = new Map<VNode, VRender>()
  private needsRerender: boolean = false
  private timer: Timer | null = null

  protected constructor(assetCacher: VAssetCacher, root: () => VNode, {fps}: CoreRenderOptions) {
    this.root = VRoot(root, this)
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

  setNeedsRerender(diff: RenderDiff) {
    let node: VNode | null = diff
    while (node !== null) {
      this.cachedRenders.delete(diff)
      node = node.parent
    }
    this.needsRerender = true
  }

  reroot(root: () => VNode) {
    this.root = VRoot(root, this)
    this.cachedRenders.clear()
    this.needsRerender = true
  }

  rerender() {
    this.clear()
    this.writeRender(this.renderNode(this.root))
  }

  protected abstract clear(): void

  protected abstract writeRender(render: VRender): void

  protected renderNode(node: VNode): VRender {
    if (this.cachedRenders.has(node)) {
      return this.cachedRenders.get(node)!
    } else {
      const render = this.renderNodeImpl(node)
      this.cachedRenders.set(node, render)
      return render
    }
  }

  protected abstract renderNodeImpl(node: VNode): VRender

  dispose() {
    if (this.timer !== null) {
      this.stop()
    }
  }
}
