import { BoundingBox, Bounds, Color, ParentBounds, Rectangle, Size, VView, VNode } from 'core/view'
import { CoreRenderOptions, DEFAULT_CORE_RENDER_OPTIONS, DEFAULT_COLUMN_SIZE, Renderer } from 'core/renderer'
import { VComponent, VRoot } from 'core/component'
import { assert, Key, Strings } from '@raycenity/misc-ts'
import { BorderStyle } from 'core/view/border-style'
import type { DisplayObject } from 'pixi.js'

type Timer = NodeJS.Timer

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
  rect: Rectangle | null
}

interface CachedRenderInfo {
  parentBounds: ParentBounds
  siblingBounds: Rectangle | null
  parent: VView | null
}

export abstract class RendererImpl<VRender, AssetCacher extends CoreAssetCacher> implements Renderer {
  private readonly defaultFps: number
  root: VComponent | null = null
  protected readonly assets: AssetCacher

  private readonly cachedRenders: Map<VView, VRenderBatch<VRender> & CachedRenderInfo> = new Map()
  private needsRerender: boolean = false
  private timer: Timer | null = null
  private isVisible: boolean = false

  protected constructor (assetCacher: AssetCacher, { fps }: CoreRenderOptions) {
    this.defaultFps = fps ?? DEFAULT_CORE_RENDER_OPTIONS.fps
    this.assets = assetCacher
  }

  protected finishInit (mkRoot: () => VComponent): void {
    const root = VRoot(this, mkRoot)
    assert(this.root === root, 'sanity check failed: root component assigned during build tree doesn\'t match root component from VRoot')
    assert(this.root.node !== null, 'sanity check failed: root\'s node not created after VRoot')
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
    this.forceRerender()
    this.start()
  }

  hide (): void {
    this.stop()
    this.clear()
    this.isVisible = false
  }

  invalidate (view: VView): void {
    let nextView: VView | null = view
    while (nextView !== null) {
      if (this.cachedRenders.has(nextView)) {
        const view = nextView
        nextView = this.cachedRenders.get(nextView)!.parent
        this.cachedRenders.delete(view)
      } else {
        break
      }
    }
    this.needsRerender = true
  }

  reroot<Props> (props?: Props, mkRoot?: (props: Props) => VView): void {
    if (props !== undefined) {
      this.root!.props = props
    }
    if (mkRoot !== undefined) {
      this.root!.construct = mkRoot
    }
    VComponent.update(this.root!, mkRoot !== undefined ? 'set-root' : props !== undefined ? 'set-props' : 'manual')
    this.cachedRenders.clear()
    this.needsRerender = true
  }

  forceRerender (): void {
    this.needsRerender = false
    this.clear()
    assert(this.root!.node !== null, 'sanity check failed: root not created by the time forceRender is called')
    this.writeRender(this.renderNode(null, this.getRootParentBounds(), null, this.root!.node))
  }

  abstract useInput (handler: (key: Key) => void): () => void

  protected abstract clear (): void
  protected abstract writeRender (render: VRenderBatch<VRender>): void
  protected abstract getRootDimensions (): {
    boundingBox: BoundingBox
    columnSize?: Size
  }
  protected abstract renderText (bounds: BoundingBox, columnSize: Size, wrapMode: 'word' | 'char' | 'clip' | undefined, color: Color | null, text: string, node: VView): VRender
  protected abstract renderSolidColor (rect: Rectangle, columnSize: Size, color: Color, node: VView): VRender
  protected abstract renderBorder (rect: Rectangle, columnSize: Size, color: Color | null, borderStyle: BorderStyle, node: VView): VRender
  protected abstract renderImage (bounds: BoundingBox, columnSize: Size, src: string, node: VView): { render: VRender, size: Size }
  protected abstract renderVectorImage (bounds: BoundingBox, columnSize: Size, src: string, node: VView): { render: VRender, size: Size }
  protected abstract renderPixi (bounds: BoundingBox, columnSize: Size, pixi: DisplayObject | 'terminal', getSize: ((pixi: DisplayObject, bounds: BoundingBox, columnSize: Size) => Size) | undefined, node: VView): { render: VRender, size: Size | null }

  protected renderNode (parent: VView | null, parentBounds: ParentBounds, siblingBounds: Rectangle | null, node: VNode): VRenderBatch<VRender> {
    if (node.type === 'component') {
      assert(node.node !== null, 'sanity check: renderNode called on node with an uninitialized child')
      return this.renderNode(parent, parentBounds, siblingBounds, node.node)
    }

    if (this.cachedRenders.has(node)) {
      const cachedRender = this.cachedRenders.get(node)!
      if (
        ParentBounds.equals(cachedRender.parentBounds, parentBounds) &&
        Rectangle.equals(cachedRender.siblingBounds, siblingBounds)
      ) {
        return cachedRender
      } else {
        this.cachedRenders.delete(node)
      }
    }
    const render: VRenderBatch<VRender> & CachedRenderInfo = this.renderNodeImpl(parentBounds, siblingBounds, node) as any
    render.parentBounds = parentBounds
    render.siblingBounds = siblingBounds
    render.parent = parent
    this.cachedRenders.set(node, render)
    return render
  }

  private getRootParentBounds (): ParentBounds {
    return {
      ...this.getRootDimensions(),
      columnSize: DEFAULT_COLUMN_SIZE,
      sublayout: {}
    }
  }

  private renderNodeImpl (parentBounds: ParentBounds, siblingBounds: Rectangle | null, node: VView): VRenderBatch<VRender> {
    if (node.visible === false) {
      return { rect: null }
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
          const childRender = this.renderNode(node, bounds2, lastChild?.rect ?? null, VNode.view(child))
          children.push(childRender)
          lastChild = childRender
        }

        // Reverse so renders earlier are rendered above
        children.reverse()

        // Merge child renders
        const mergedRender: VRenderBatch<VRender> = { rect: null }
        for (const child of children) {
          mergedRender.rect = Rectangle.union(mergedRender.rect, child.rect)
          for (const [zString, render] of Object.entries(child)) {
            let zPosition = Number(zString)
            if (!isNaN(zPosition)) {
              while (zPosition in mergedRender) {
                zPosition += Bounds.DELTA_Z
              }
              mergedRender[zPosition] = render
            }
          }
        }
        return mergedRender
      }
      case 'text': {
        const lines = node.text.split('\n')
        const rect = BoundingBox.toRectangle(bounds, {
          width: Math.max(0, ...lines.map(Strings.width)),
          height: lines.length
        })
        return {
          rect,
          [bounds.z]: this.renderText(bounds, parentBounds.columnSize, node.wrapMode, node.color, node.text, node)
        }
      }
      case 'color': {
        const inferredBounds = {
          ...bounds,
          width: bounds.width ?? parentBounds.boundingBox.width ?? siblingBounds?.width,
          height: bounds.height ?? parentBounds.boundingBox.height ?? siblingBounds?.height
        }
        if (inferredBounds.width === undefined || inferredBounds.height === undefined) {
          throw new Error('Cannot infer width or height for color node')
        }
        const rect = BoundingBox.toRectangle(inferredBounds as BoundingBox & Size)
        return {
          rect,
          [bounds.z]: this.renderSolidColor(rect, parentBounds.columnSize, node.color, node)
        }
      }
      case 'border': {
        const inferredBounds = {
          ...bounds,
          width: bounds.width ?? parentBounds.boundingBox.width ?? siblingBounds?.width,
          height: bounds.height ?? parentBounds.boundingBox.height ?? siblingBounds?.height
        }
        if (inferredBounds.width === undefined || inferredBounds.height === undefined) {
          throw new Error('Cannot infer width or height for border node')
        }
        const rect = BoundingBox.toRectangle(inferredBounds as BoundingBox & Size)
        return {
          rect,
          [bounds.z]: this.renderBorder(rect, parentBounds.columnSize, node.color, node.style, node)
        }
      }
      case 'source': {
        const extension = node.src.split('.').pop()
        switch (extension) {
          case 'png':
          case 'jpg':
          case 'jpeg':
          case 'gif': {
            const { render, size } = this.renderImage(bounds, parentBounds.columnSize, node.src, node)
            const rect = BoundingBox.toRectangle(bounds, size)
            return {
              rect,
              [bounds.z]: render
            }
          }
          case 'svg': {
            const { render, size } = this.renderVectorImage(bounds, parentBounds.columnSize, node.src, node)
            const rect = BoundingBox.toRectangle(bounds, size)
            return {
              rect,
              [bounds.z]: render
            }
          }
          case undefined:
            throw new Error('source must have an extension to determine the filetype')
          default:
            throw new Error(`unsupported source extension: ${extension}`)
        }
      }
      case 'pixi': {
        const inferredBounds = {
          ...bounds,
          width: bounds.width ?? parentBounds.boundingBox.width ?? siblingBounds?.width,
          height: bounds.height ?? parentBounds.boundingBox.height ?? siblingBounds?.height
        }
        const { render, size } = this.renderPixi(inferredBounds, parentBounds.columnSize, node.pixi, node.getSize, node)
        const rect = size !== null ? BoundingBox.toRectangle(bounds, size) : null
        return {
          rect,
          [bounds.z]: render
        }
      }
    }
  }

  dispose (): void {
    if (this.timer !== null) {
      this.stop()
    }

    VComponent.destroy(this.root!)
    this.root = null
  }
}
