import { BoundingBox, Bounds, LCHColor, ParentBounds, VNode } from 'core/vdom'
import { CoreRenderOptions, Renderer } from 'core/renderer'
import { VComponent, VRoot } from 'core/component'
import { Key } from '@raycenity/misc-ts'

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

export interface VRenderBatch<VRender> {
  [zPosition: number]: VRender
  bounds: BoundingBox | null
}

interface CachedRenderInfo {
  parentBounds: ParentBounds
  siblingBounds: BoundingBox | null
}

export abstract class RendererImpl<VRender, AssetCacher extends CoreAssetCacher> implements Renderer {
  static readonly DEFAULT_FPS: number = 20
  static readonly DEFAULT_COLUMN_SIZE: { width: number, height: number } = {
    width: 7,
    height: 14
  }

  private readonly defaultFps: number
  private root: VNode | null = null
  rootComponent: VComponent | null = null
  protected readonly assets: AssetCacher

  private readonly cachedRenders: Map<VNode, VRenderBatch<VRender> & CachedRenderInfo> = new Map()
  private needsRerender: boolean = false
  private timer: Timer | null = null
  private isVisible: boolean = false

  protected constructor (assetCacher: AssetCacher, { fps }: CoreRenderOptions) {
    this.defaultFps = fps ?? RendererImpl.DEFAULT_FPS
    this.assets = assetCacher
  }

  protected finishInit (root: () => VNode): void {
    this.root = VRoot(this, root)
    if (this.rootComponent?.node !== this.root) {
      throw new Error('sanity check failed: root component node does not match root node')
    }
  }

  start (fps?: number): void {
    if (this.timer !== null) {
      throw new Error('Renderer is already running')
    }

    this.timer = setInterval(() => {
      if (this.needsRerender && this.isVisible) {
        this.forceRerender()
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
    let node: VNode | 'none' = diff
    while (node !== 'none') {
      this.cachedRenders.delete(node)
      node = node.parent!
    }
    this.needsRerender = true
  }

  reroot (root?: () => VNode): void {
    if (root !== undefined) {
      this.rootComponent!.construct = root
    }
    VComponent.update(this.rootComponent!)
    this.cachedRenders.clear()
    this.needsRerender = true
  }

  forceRerender (): void {
    this.needsRerender = false
    this.clear()
    this.writeRender(this.renderNode(this.getRootParentBounds(), null, this.root!))
  }

  abstract useInput (handler: (key: Key) => void): () => void

  protected abstract clear (): void
  protected abstract writeRender (render: VRenderBatch<VRender>): void
  protected abstract getRootDimensions (): {
    boundingBox: BoundingBox
    columnSize?: { width: number, height: number }
  }
  protected abstract renderText (bounds: BoundingBox, wrapMode: 'word' | 'char' | 'clip' | undefined, text: string, node: VNode): VRender
  protected abstract renderSolidColor (bounds: BoundingBox, color: LCHColor, node: VNode): VRender
  protected abstract renderImage (bounds: BoundingBox, src: string, node: VNode): VRender
  protected abstract renderVectorImage (bounds: BoundingBox, src: string, node: VNode): VRender

  protected renderNode (parentBounds: ParentBounds, siblingBounds: BoundingBox | null, node: VNode): VRenderBatch<VRender> {
    if (this.cachedRenders.has(node)) {
      const cachedRender = this.cachedRenders.get(node)!
      if (
        ParentBounds.equals(cachedRender.parentBounds, parentBounds) &&
        BoundingBox.equals(cachedRender.siblingBounds, siblingBounds)
      ) {
        return cachedRender
      } else {
        this.cachedRenders.delete(node)
      }
    }
    const render: VRenderBatch<VRender> & CachedRenderInfo = this.renderNodeImpl(parentBounds, siblingBounds, node) as any
    render.parentBounds = parentBounds
    render.siblingBounds = siblingBounds
    this.cachedRenders.set(node, render)
    return render
  }

  private getRootParentBounds (): ParentBounds {
    return {
      ...this.getRootDimensions(),
      columnSize: RendererImpl.DEFAULT_COLUMN_SIZE,
      sublayout: {}
    }
  }

  private renderNodeImpl (parentBounds: ParentBounds, siblingBounds: BoundingBox | null, node: VNode): VRenderBatch<VRender> {
    if (node.visible === false) {
      return { bounds: null }
    }

    const bounds = (node.bounds ?? Bounds.DEFAULT)(parentBounds, siblingBounds)

    switch (node.type) {
      case 'box': {
        const bounds2: ParentBounds = {
          boundingBox: bounds,
          sublayout: node.sublayout ?? {},
          columnSize: parentBounds.columnSize
        }

        // Render children
        const children = []
        let lastChild = null
        for (const child of node.children) {
          const childRender = this.renderNode(bounds2, lastChild?.bounds ?? null, child)
          children.push(childRender)
          lastChild = childRender
        }

        // Merge child renders
        const render: VRenderBatch<VRender> = { bounds }
        for (const child of children) {
          for (const [zString, render] of Object.entries(child)) {
            let zPosition = Number(zString)
            while (zPosition in render) {
              zPosition += Bounds.DELTA_Z
            }
            render[zPosition] = render
          }
        }
        return render
      }
      case 'text':
        return {
          bounds,
          [bounds.z]: this.renderText(bounds, node.wrapMode, node.text, node)
        }
      case 'color':
        return {
          bounds,
          [bounds.z]: this.renderSolidColor(bounds, node.color, node)
        }
      case 'source': {
        const extension = node.src.split('.').pop()
        switch (extension) {
          case 'png':
          case 'jpg':
          case 'jpeg':
          case 'gif':
            return {
              bounds,
              [bounds.z]: this.renderImage(bounds, node.src, node)
            }
          case 'svg':
            return {
              bounds,
              [bounds.z]: this.renderVectorImage(bounds, node.src, node)
            }
          case undefined:
            throw new Error('source must have an extension to determine the filetype')
          default:
            throw new Error(`unsupported source extension: ${extension}`)
        }
      }
    }
  }

  dispose (): void {
    if (this.timer !== null) {
      this.stop()
    }

    VComponent.destroy(this.rootComponent!)
    this.rootComponent = null
  }
}
