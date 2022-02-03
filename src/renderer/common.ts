import { VNode } from 'core/vdom'
import { CoreRenderOptions, Renderer } from 'core/renderer'
import { VComponent, VRoot } from 'core/component'
import { Key } from 'misc'

type Timer = NodeJS.Timer

export type RenderDiff = VNode

export abstract class CoreAssetCacher {
  private readonly assets: Map<string, any> = new Map()

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
        void construct(path).then(image => {
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
  private root: VNode | null = null
  rootComponent: VComponent | null = null
  protected readonly assets: VAssetCacher

  private readonly cachedRenders: Map<VNode, VRender> = new Map<VNode, VRender>()
  private needsRerender: boolean = false
  private timer: Timer | null = null
  private isVisible: boolean = true

  protected constructor (assetCacher: VAssetCacher, { fps }: CoreRenderOptions) {
    this.defaultFps = fps ?? RendererImpl.DEFAULT_FPS
    this.assets = assetCacher
  }

  protected finishInit (root: () => VNode): void {
    this.root = VRoot(root, this)
    if (this.rootComponent?.node !== this.root) {
      throw new Error('sanity check failed: root component node does not match root node')
    }
  }

  start (fps?: number): void {
    if (this.timer !== null) {
      throw new Error('Renderer is already running')
    }

    this.timer = setInterval(() => {
      if (this.needsRerender) {
        this.rerender()
      }
    }, 1 / (fps ?? this.defaultFps))
  }

  stop (): void {
    if (this.timer === null) {
      throw new Error('Renderer is not running')
    }

    clearInterval(this.timer)
    this.timer = null
  }

  show (): void {
    this.isVisible = true
    this.start()
  }

  hide (): void {
    this.stop()
    this.clear()
    this.isVisible = false
  }

  setNeedsRerender (diff: RenderDiff): void {
    let node: VNode | null = diff
    while (node !== null) {
      this.cachedRenders.delete(diff)
      node = node.parent
    }
    this.needsRerender = true
  }

  reroot (root: () => VNode): void {
    VComponent.runDestroys(this.rootComponent!)
    this.rootComponent = null

    this.root = VRoot(root, this)
    this.cachedRenders.clear()
    this.needsRerender = true
  }

  rerender (): void {
    if (this.isVisible) {
      this.needsRerender = false
      this.clear()
      this.writeRender(this.renderNode(this.root!))
    }
  }

  abstract useInput (handler: (key: Key) => void): () => void

  protected abstract clear (): void

  protected abstract writeRender (render: VRender): void

  protected renderNode (node: VNode): VRender {
    if (this.cachedRenders.has(node)) {
      return this.cachedRenders.get(node)!
    } else {
      const render = this.renderNodeImpl(node)
      this.cachedRenders.set(node, render)
      return render
    }
  }

  protected abstract renderNodeImpl (node: VNode): VRender

  dispose (): void {
    if (this.timer !== null) {
      this.stop()
    }

    VComponent.runDestroys(this.rootComponent!)
    this.rootComponent = null
  }
}
