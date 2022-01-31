import * as PIXI from 'pixi.js'
import { VNode } from 'core/vdom'
import stringWidth from 'string-width'
import { CoreRenderOptions } from 'core/renderer'
import { CoreAssetCacher, RendererImpl } from 'renderer/common'

interface VRender {
  pixi: PIXI.DisplayObject | null
  width: number
  height: number
}

export interface BrowserRenderOptions extends CoreRenderOptions, PIXI.IApplicationOptions {
  container?: HTMLElement
  em?: number
}

class AssetCacher extends CoreAssetCacher {
  getImage (path: string): PIXI.Texture {
    return this.get(path, PIXI.Texture.from)
  }
}

export class BrowserRendererImpl extends RendererImpl<VRender, AssetCacher> {
  static readonly EM: number = 16

  private readonly canvas: PIXI.Application

  private readonly em: number

  constructor (root: () => VNode, opts: BrowserRenderOptions = {}) {
    super(new AssetCacher(), root, opts)

    const container = opts.container ?? document.body
    this.canvas = new PIXI.Application({
      width: container.clientWidth,
      height: container.clientHeight,
      backgroundColor: 0xffffff,
      antialias: true,
      resolution: 1,
      ...opts
    })
    this.em = opts.em ?? BrowserRendererImpl.EM
  }

  protected override clear (): void {
    this.canvas.stage.removeChildren()
  }

  protected override writeRender (render: VRender): void {
    if (render.pixi !== null) {
      this.canvas.stage.addChild(render.pixi)
    }
  }

  protected override renderNodeImpl (node: VNode): VRender {
    if (VNode.isText(node)) {
      return this.renderText(node.text)
    } else if (VNode.isBox(node)) {
      const {
        visible,
        direction,
        width,
        height,
        marginLeft,
        marginTop,
        marginRight,
        marginBottom,
        paddingLeft,
        paddingTop,
        paddingRight,
        paddingBottom
      } = node.box
      if (visible === false) {
        return {
          pixi: null,
          width: 0,
          height: 0
        }
      }

      // Render children
      const children = this.renderDivChildren(node.children, direction)
      const pixi = children.pixi

      // Add padding
      if (paddingLeft !== undefined) {
        pixi.x += paddingLeft
      }
      if (paddingTop !== undefined) {
        pixi.y += paddingTop
      }

      // Clip to get correct size
      if (width !== undefined) {
        const childWidth = width - (paddingLeft ?? 0) - (paddingRight ?? 0)
        if (children.width > childWidth) {
          pixi.width = childWidth
        }
      }
      if (height !== undefined) {
        const childHeight = height - (paddingTop ?? 0) - (paddingBottom ?? 0)
        if (children.height > childHeight) {
          pixi.height = childHeight
        }
      }

      // Add margin
      if (marginLeft !== undefined) {
        pixi.x += marginLeft
      }
      if (marginTop !== undefined) {
        pixi.y += marginTop
      }

      return {
        pixi,
        width: (width ?? (children.width + (paddingLeft ?? 0) + (paddingRight ?? 0))) + (marginLeft ?? 0) + (marginRight ?? 0),
        height: (height ?? (children.height + (paddingTop ?? 0) + (paddingBottom ?? 0))) + (marginTop ?? 0) + (marginBottom ?? 0)
      }
    } else if (VNode.isImage(node)) {
      const {
        visible,
        width,
        height
      } = node.image
      if (visible === false) {
        return {
          pixi: null,
          width: 0,
          height: 0
        }
      }
      const image = this.renderImage(node.path)

      if (width !== undefined) {
        image.width = width
      }
      if (height !== undefined) {
        image.height = height
      }

      return {
        pixi: image,
        width: width ?? image.width,
        height: height ?? image.height
      }
    } else {
      throw new Error('Unhandled node type')
    }
  }

  private renderText (text: string): VRender {
    const lines = text.split('\n')
    const width = lines.reduce((max, line) => Math.max(max, stringWidth(line)), 0)
    const pixi = new PIXI.Text(text, {
      fontFamily: 'monospace',
      fontSize: this.em,
      align: 'left',
      wordWrap: false
    })
    return {
      pixi,
      width,
      height: lines.length
    }
  }

  private renderDivChildren (children: VNode[], renderDirection?: 'horizontal' | 'vertical' | null): VRender & { pixi: PIXI.Container } {
    const container = new PIXI.Container()
    let width = 0
    let height = 0
    if (renderDirection === 'vertical') {
      for (const child of children) {
        const render = this.renderNodeImpl(child)
        if (render.pixi !== null) {
          render.pixi.y = height
          container.addChild(render.pixi)
        }
        width = Math.max(width, render.width)
        height += render.height
      }
    } else if (renderDirection === 'horizontal') {
      for (const child of children) {
        const render = this.renderNodeImpl(child)
        if (render.pixi !== null) {
          render.pixi.x = width
          container.addChild(render.pixi)
        }
        width += render.width
        height = Math.max(height, render.height)
      }
    } else {
      for (const child of children) {
        const render = this.renderNodeImpl(child)
        if (render.pixi !== null) {
          container.addChild(render.pixi)
        }
        width = Math.max(width, render.width)
        height = Math.max(height, render.height)
      }
    }
    return {
      pixi: container,
      width,
      height
    }
  }

  private renderImage (path: string): PIXI.Sprite {
    const image = new PIXI.Sprite(this.assets.getImage(path))
    // noinspection JSDeprecatedSymbols IntelliJ bug
    image.anchor.set(0, 0)
    return image
  }

  override useInput (handler: (key: string, event: KeyboardEvent) => void): () => void {
    function listener (key: KeyboardEvent): void {
      handler(key.key, key)
    }
    document.body.addEventListener('keypress', listener)
    return () => {
      document.body.removeEventListener('keypress', listener)
    }
  }

  override start (fps?: number): void {
    super.start(fps)
    this.canvas.start()
  }

  override stop (): void {
    super.stop()
    this.canvas.stop()
  }

  override dispose (): void {
    super.dispose()
    this.canvas.destroy()
  }
}
