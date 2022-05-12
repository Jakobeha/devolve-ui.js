import { BorderStyle, BoundingBox, Color, Rectangle, Size } from 'core/view'
import { CoreRenderOptions, DEFAULT_COLUMN_SIZE } from 'core/renderer'
import { CoreAssetCacher, RendererImpl, VRenderBatch } from 'renderer/common'
import { Key, Strings } from '@raycenity/misc-ts'
import type { Application, DisplayObject, IApplicationOptions, Sprite, Texture } from 'pixi.js'
import { VComponent } from 'core/component'

declare global {
  const PIXI: typeof import('pixi.js')
}

type VRender = DisplayObject

export interface BrowserRenderOptions extends CoreRenderOptions, IApplicationOptions {
  container?: HTMLElement
  em?: number
}

class AssetCacher extends CoreAssetCacher {
  getImage (path: string): Texture {
    return this.get(path, PIXI.Texture.from)
  }
}

export class BrowserRendererImpl extends RendererImpl<VRender, AssetCacher> {
  private readonly canvas: Application

  private readonly em: number | null

  constructor (root: () => VComponent, opts: BrowserRenderOptions = {}) {
    super(new AssetCacher(), opts)

    const commonOpts: IApplicationOptions = {
      antialias: true,
      resolution: 1,
      ...opts
    }
    if (opts.view === undefined) {
      const container = opts.container ?? document.body
      this.canvas = new PIXI.Application({
        width: container.clientWidth,
        height: container.clientHeight,
        ...commonOpts
      })
      container.appendChild(this.canvas.view)
    } else {
      // Already has container, width, and height
      this.canvas = new PIXI.Application(commonOpts)
    }
    this.em = opts.em ?? null

    this.finishInit(root)
  }

  protected override clear (): void {
    this.canvas.stage.removeChildren()
  }

  protected override writeRender (render: VRenderBatch<VRender>): void {
    const collapsed = Object.entries(render)
      .filter(([key]) => !isNaN(parseFloat(key)))
      .sort(([lhs], [rhs]) => Number(lhs) - Number(rhs))
      .map(([, value]) => value)
    this.canvas.stage.addChild(...collapsed)
  }

  protected override getRootDimensions (): {
    boundingBox: BoundingBox
    columnSize?: Size
  } {
    const columnSize = this.em !== null
      ? {
          width: this.em / 2,
          height: this.em
        }
      : DEFAULT_COLUMN_SIZE
    return {
      boundingBox: {
        x: 0,
        y: 0,
        z: 0,
        anchorX: 0,
        anchorY: 0,
        width: this.canvas.view.width / columnSize.width,
        height: this.canvas.view.height / columnSize.height
      },
      columnSize
    }
  }

  protected override renderText (bounds: BoundingBox, columnSize: Size, wrapMode: 'word' | 'char' | 'clip' | undefined, color: Color | null, text: string): VRender {
    if (bounds.width !== undefined) {
      if (wrapMode === 'clip') {
        // Remove clipped characters
        text = text.split('\n').map(line => Strings.truncateEnd(line, bounds.width!)).join('\n')
      } else if (wrapMode === undefined) {
        // Warn if characters go past end
        if (text.split('\n').some(line => Strings.width(line) > bounds.width!)) {
          console.warn(`wrap is undefined but text goes path width (text = ${text})`)
        }
      }
    }

    const render = new PIXI.Text(text, {
      fontFamily: 'monospace',
      fontSize: this.em ?? columnSize.height,
      fill: color === null ? 0x000000 : color2Number(color),
      align: 'left',
      wordWrap: wrapMode === 'word',
      wordWrapWidth: wrapMode === 'word' ? bounds.width : undefined,
      lineHeight: this.em ?? columnSize.height
    })

    transformSpriteRender(render, bounds, columnSize)

    return render
  }

  protected override renderSolidColor (rect: Rectangle, columnSize: Size, color: Color): VRender {
    const pixiColor = new PIXI.Graphics()
    pixiColor.beginFill(color2Number(color))
    pixiColor.drawRect(
      rect.left * columnSize.width,
      rect.top * columnSize.height,
      rect.width * columnSize.width,
      rect.height * columnSize.height
    )
    return pixiColor
  }

  protected override renderBorder (rect: Rectangle, columnSize: Size, color: Color | null, borderStyle: BorderStyle): VRender {
    const pixiColor = new PIXI.Graphics()
    pixiColor.lineStyle(borderStyle === 'thick' ? 2 : 1, color2Number(color ?? Color('black')))
    switch (borderStyle) {
      case 'single':
      case 'ascii':
        pixiColor.drawRect(
          rect.left * columnSize.width,
          rect.top * columnSize.height,
          rect.width * columnSize.width,
          rect.height * columnSize.height
        )
        break
      case 'card':
        pixiColor.drawRect(
          (rect.left - 0.125) * columnSize.width,
          rect.top * columnSize.height,
          (rect.width + 0.25) * columnSize.width,
          rect.height * columnSize.height
        )
        pixiColor.drawRect(
          (rect.left + 0.125) * columnSize.width,
          rect.top * columnSize.height,
          (rect.width + 0.25) * columnSize.width,
          rect.height * columnSize.height
        )
        break
      case 'double':
        pixiColor.drawRect(
          (rect.left - 0.125) * columnSize.width,
          (rect.top - 0.0625) * columnSize.height,
          (rect.width + 0.25) * columnSize.width,
          (rect.height + 0.125) * columnSize.height
        )
        pixiColor.drawRect(
          (rect.left + 0.125) * columnSize.width,
          (rect.top + 0.0625) * columnSize.height,
          (rect.width + 0.25) * columnSize.width,
          (rect.height + 0.125) * columnSize.height
        )
        break
      case 'thick':
        pixiColor.drawRect(
          (rect.left - 0.125) * columnSize.width,
          (rect.top - 0.0625) * columnSize.height,
          (rect.width + 0.25) * columnSize.width,
          (rect.height + 0.125) * columnSize.height
        )
        break
      case 'rounded':
        pixiColor.drawRoundedRect(
          rect.left * columnSize.width,
          rect.top * columnSize.height,
          rect.width * columnSize.width,
          rect.height * columnSize.height,
          Math.min(columnSize.width, columnSize.height)
        )
        break
      case 'dashed':
      case 'ascii-dashed':
        console.warn('TODO: dashed border style not supported by Pixi renderer')
        pixiColor.drawRect(
          rect.left * columnSize.width,
          rect.top * columnSize.height,
          rect.width * columnSize.width,
          rect.height * columnSize.height
        )
        break
    }
    return pixiColor
  }

  protected override renderImage (bounds: BoundingBox, columnSize: Size, path: string): { render: VRender, size: Size } {
    const image = this.assets.getImage(path)
    const render = new PIXI.Sprite(image)

    transformSpriteRender(render, bounds, columnSize)

    const size: Size = {
      width: image.width / columnSize.width,
      height: image.height / columnSize.height
    }
    return { render, size }
  }

  protected override renderVectorImage (bounds: BoundingBox, columnSize: Size, path: string): { render: VRender, size: Size } {
    // TODO
    return null as any
  }

  protected override renderPixi (bounds: BoundingBox, columnSize: Size, pixi: DisplayObject | 'terminal', getSize: ((pixi: DisplayObject, bounds: BoundingBox, columnSize: Size) => Size) | undefined): { render: VRender, size: Size | null } {
    if (pixi === 'terminal') {
      throw new Error('pixi DisplayObject and getSize should not be null in browser')
    }

    return {
      render: pixi,
      size: getSize?.(pixi, bounds, columnSize) ?? null
    }
  }

  override useInput (handler: (key: Key) => void): () => void {
    function listener (key: KeyboardEvent): void {
      handler(Key.fromKeyboardEvent(key))
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

function color2Number (color: Color): number {
  const { red, green, blue } = Color.toRGB(color)
  return PIXI.utils.rgb2hex([red, green, blue])
}

function transformSpriteRender (render: Sprite, bounds: BoundingBox, columnSize: Size): void {
  render.position.set(bounds.x * columnSize.width, bounds.y * columnSize.height)
  render.anchor.set(bounds.anchorX, bounds.anchorY)
  if (bounds.width !== undefined) {
    render.width = bounds.width * columnSize.width
  }
  if (bounds.height !== undefined) {
    render.height = bounds.height * columnSize.height
  }
}
