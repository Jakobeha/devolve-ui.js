import { BoundingBox, Bounds, Color, DelayedSubLayout, ParentBounds, Rectangle, Size, VView, VNode } from 'core/view'
import { CoreRenderOptions, DEFAULT_CORE_RENDER_OPTIONS, DEFAULT_COLUMN_SIZE, Renderer } from 'core/renderer'
import { doLogRender, VComponent, VRoot } from 'core/component'
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
  parent: number
}

export abstract class RendererImpl<VRender, AssetCacher extends CoreAssetCacher> implements Renderer {
  private readonly defaultFps: number
  root: VComponent | null = null
  protected readonly assets: AssetCacher

  private readonly cachedRenders: Map<number, VRenderBatch<VRender> & CachedRenderInfo> = new Map()
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

  invalidate (node: VNode): void {
    const view = VNode.view(node)

    RendererImpl.logRender('invalidate', view)
    let nextViewId: number = view.id
    while (nextViewId !== -1) {
      if (this.cachedRenders.has(nextViewId)) {
        const viewId = nextViewId
        nextViewId = this.cachedRenders.get(viewId)!.parent
        this.cachedRenders.delete(viewId)
        RendererImpl.logRender('- found ->', nextViewId)
      } else {
        RendererImpl.logRender('- not found')
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
  /** Can mutate `render` if it's faster */
  protected abstract clipRender (clipRect: Rectangle, columnSize: Size, render: VRender): VRender
  protected abstract renderText (bounds: BoundingBox, columnSize: Size, wrapMode: 'word' | 'char' | 'clip' | undefined, color: Color | null, text: string, node: VView): VRender
  protected abstract renderSolidColor (rect: Rectangle, columnSize: Size, color: Color, node: VView): VRender
  protected abstract renderBorder (rect: Rectangle, columnSize: Size, color: Color | null, borderStyle: BorderStyle, node: VView): VRender
  protected abstract renderImage (bounds: BoundingBox, columnSize: Size, src: string, node: VView): { render: VRender, size: Size }
  protected abstract renderVectorImage (bounds: BoundingBox, columnSize: Size, src: string, node: VView): { render: VRender, size: Size }
  protected abstract renderPixi (bounds: BoundingBox, columnSize: Size, pixi: DisplayObject | 'terminal', getSize: ((pixi: DisplayObject, bounds: BoundingBox, columnSize: Size) => Size) | undefined, node: VView): { render: VRender, size: Size | null }

  protected renderNode (parent: VView | null, parentBounds: ParentBounds, siblingBounds: Rectangle | null, node: VNode): VRenderBatch<VRender> {
    const view = VNode.view(node)

    RendererImpl.logRender('render', view, 'parent', parent)
    if (this.cachedRenders.has(view.id)) {
      RendererImpl.logRender('- cached')
      const cachedRender = this.cachedRenders.get(view.id)!
      if (
        ParentBounds.equals(cachedRender.parentBounds, parentBounds) &&
        Rectangle.equals(cachedRender.siblingBounds, siblingBounds)
      ) {
        return cachedRender
      } else {
        this.cachedRenders.delete(view.id)
      }
    }
    const render: VRenderBatch<VRender> & CachedRenderInfo = this.renderViewImpl(parentBounds, siblingBounds, view) as any
    render.parentBounds = parentBounds
    render.siblingBounds = siblingBounds
    render.parent = parent?.id ?? -1
    this.cachedRenders.set(view.id, render)
    return render
  }

  private getRootParentBounds (): ParentBounds {
    return {
      ...this.getRootDimensions(),
      columnSize: DEFAULT_COLUMN_SIZE,
      sublayout: {}
    }
  }

  private renderViewImpl (parentBounds: ParentBounds, siblingBounds: Rectangle | null, view: VView): VRenderBatch<VRender> {
    if (view.visible === false) {
      return { rect: null }
    }

    const bounds = (view.bounds ?? Bounds.DEFAULT)(parentBounds, siblingBounds)

    switch (view.type) {
      case 'box': {
        const bounds2: ParentBounds = {
          boundingBox: bounds,
          sublayout: DelayedSubLayout.resolve(view.sublayout ?? {}, bounds, parentBounds, siblingBounds),
          columnSize: parentBounds.columnSize
        }

        // Render children
        const children = []
        let lastChild = null
        for (const child of view.children) {
          const childRender = this.renderNode(view, bounds2, lastChild?.rect ?? null, child)
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

        // Clip if necessary
        if (view.clip === true) {
          // Not sure whether to use mergedRender.rect or Infinity
          // mergedRender.rect seems more consistent wrt both negative and positive offsets being clipped with auto size,
          // and you can ignore this behavior simply by nesting the clipping box in another offset box
          const clipRect = BoundingBox.toRectangle(bounds, {
            width: bounds.width ?? mergedRender.rect?.width ?? 0,
            height: bounds.height ?? mergedRender.rect?.height ?? 0
          })
          if (view.extend === true) {
            mergedRender.rect = clipRect
          } else {
            mergedRender.rect = Rectangle.intersection(mergedRender.rect, clipRect)
          }
          for (const zString in mergedRender) {
            const zPosition = Number(zString)
            if (!isNaN(zPosition)) {
              mergedRender[zPosition] = this.clipRender(clipRect, parentBounds.columnSize, mergedRender[zPosition])
            }
          }
        } else if (view.extend === true) {
          if (mergedRender.rect !== null && bounds.width !== undefined && mergedRender.rect.width < bounds.width) {
            mergedRender.rect.width = bounds.width
          }
          if (mergedRender.rect !== null && bounds.height !== undefined && mergedRender.rect.height < bounds.height) {
            mergedRender.rect.height = bounds.height
          }
        }

        return mergedRender
      }
      case 'text': {
        const lines = view.text.split('\n')
        const rect = BoundingBox.toRectangle(bounds, {
          width: Math.max(0, ...lines.map(Strings.width)),
          height: lines.length
        })
        return {
          rect,
          [bounds.z]: this.renderText(bounds, parentBounds.columnSize, view.wrapMode, view.color, view.text, view)
        }
      }
      case 'color': {
        const inferredBounds = {
          ...bounds,
          width: bounds.width ?? parentBounds.boundingBox.width ?? siblingBounds?.width,
          height: bounds.height ?? parentBounds.boundingBox.height ?? siblingBounds?.height
        }
        if (inferredBounds.width === undefined || inferredBounds.height === undefined) {
          throw new Error('Cannot infer width or height for color view')
        }
        const rect = BoundingBox.toRectangle(inferredBounds as BoundingBox & Size)
        return {
          rect,
          [bounds.z]: this.renderSolidColor(rect, parentBounds.columnSize, view.color, view)
        }
      }
      case 'border': {
        const inferredBounds = {
          ...bounds,
          width: bounds.width ?? parentBounds.boundingBox.width ?? siblingBounds?.width,
          height: bounds.height ?? parentBounds.boundingBox.height ?? siblingBounds?.height
        }
        if (inferredBounds.width === undefined || inferredBounds.height === undefined) {
          throw new Error('Cannot infer width or height for border view')
        }
        const rect = BoundingBox.toRectangle(inferredBounds as BoundingBox & Size)
        return {
          rect,
          [bounds.z]: this.renderBorder(rect, parentBounds.columnSize, view.color, view.style, view)
        }
      }
      case 'source': {
        const extension = view.src.split('.').pop()
        switch (extension) {
          case 'png':
          case 'jpg':
          case 'jpeg':
          case 'gif': {
            const { render, size } = this.renderImage(bounds, parentBounds.columnSize, view.src, view)
            const rect = BoundingBox.toRectangle(bounds, size)
            return {
              rect,
              [bounds.z]: render
            }
          }
          case 'svg': {
            const { render, size } = this.renderVectorImage(bounds, parentBounds.columnSize, view.src, view)
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
        const { render, size } = this.renderPixi(inferredBounds, parentBounds.columnSize, view.pixi, view.getSize, view)
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

  private static logRender (...args: any[]): void {
    if (doLogRender()) {
      console.log(...args)
    }
  }
}
