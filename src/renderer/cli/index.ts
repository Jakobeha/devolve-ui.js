import type { Interface } from 'readline'
import type { ReadStream, WriteStream } from 'tty'
import { BorderStyle, BoundingBox, Color, Rectangle, Size, VNode } from 'core/vdom'
import { CoreRenderOptions } from 'core/renderer'
import { Key, range, Strings } from '@raycenity/misc-ts'
import { terminalImage } from 'renderer/cli/terminal-image-min'
import { CoreAssetCacher, RendererImpl, VRenderBatch } from 'renderer/common'
import { chalk } from '@raycenity/chalk-cross'
import { VRender } from 'renderer/cli/VRender'
import { CharColor, TRANSPARENT } from 'renderer/cli/CharColor'
import type { DisplayObject } from 'pixi.js'

let readline: typeof import('readline')

export function initModule (imports: { readline: typeof import('readline') }): void {
  readline = imports.readline
}

export interface TerminalRenderOptions extends CoreRenderOptions {
  input?: ReadStream
  output?: WriteStream
  interact?: Interface
  /** Determines how strict to enforce character positions.
   * More strict = less screen glitches, but also slower and less compatible with stdio or older terminals.
   * Default is 'strict', use 'loose' for simple CLI apps e.g. those without images
   */
  positionStrictness?: 'strict' | 'loose'
}

class AssetCacher extends CoreAssetCacher {
  static async image (path: string, width?: number, height?: number): Promise<VRender> {
    try {
      return await terminalImage.file(path, { width, height })
    } catch (exception) {
      console.error('Failed to load image', path, exception)
      return [['?']]
    }
  }

  getImage (path: string, width?: number, height?: number): [VRender | null, (didResolve: () => void) => void] {
    return this.getAsync(path, async path => await AssetCacher.image(path, width, height))
  }
}

export class TerminalRendererImpl extends RendererImpl<VRender, AssetCacher> {
  private readonly interact: Interface
  private readonly input: ReadStream
  private readonly output: WriteStream

  private linesOutput: number = 0
  private readonly positionStrictness: 'strict' | 'loose'

  constructor (root: () => VNode, opts: TerminalRenderOptions = {}) {
    super(new AssetCacher(), opts)

    let { input, output, interact, positionStrictness } = opts

    input = input ?? process.stdin
    output = output ?? process.stdout
    interact = interact ?? readline.createInterface({ input, output, terminal: true })
    positionStrictness = positionStrictness ?? 'strict'

    this.interact = interact
    this.input = input
    this.output = output
    this.positionStrictness = positionStrictness

    // Configure input
    if (this.input.isTTY) {
      this.input.setRawMode(true)
    }
    this.input.setEncoding('utf8')
    readline.emitKeypressEvents(this.input)

    this.finishInit(root)
  }

  protected override clear (): void {
    if (this.linesOutput !== 0) {
      if (this.positionStrictness === 'loose') {
        this.output.moveCursor(0, -this.linesOutput)
        this.output.clearScreenDown()
      }
      this.linesOutput = 0
    }
  }

  protected override writeRender (render: VRenderBatch<VRender>): void {
    const lines = VRender.collapse(render)

    if (this.positionStrictness === 'strict') {
      // Clear screen and move to top left
      this.output.write('\x1b[2J')
      this.output.write('\x1b[H')
    }

    // Write lines
    lines.forEach((line, i) => {
      line.forEach((char, j) => {
        if (this.positionStrictness === 'strict') {
          // This moves the cursor to the exact location of the character so there aren't any issues
          // It's expensive but terminal emulation is really varied, especially with images,
          // and there are a lot of terminals which just don't do things the right way
          this.output.write(`\x1b[${i + 1};${j + 1}H`)
        }
        this.output.write(char)
      })

      if (this.positionStrictness === 'loose') {
        this.output.write('\n')
      }
    })
    this.linesOutput += lines.length
  }

  protected override getRootDimensions (): {
    boundingBox: BoundingBox
    columnSize?: Size
  } {
    return {
      boundingBox: {
        x: 0,
        y: 0,
        z: 0,
        anchorX: 0,
        anchorY: 0,
        width: this.output.columns,
        height: this.output.rows
      }
    }
  }

  protected override renderText (bounds: BoundingBox, columnSize: Size, wrap: 'word' | 'char' | 'clip' | undefined, color: Color | null, text: string | string[]): VRender {
    const width = bounds.width ?? Infinity
    const height = bounds.height ?? Infinity
    const input = Array.isArray(text) ? text : text.split('\n')

    const result: VRender = []
    let nextOutLine: string[] = []
    let nextOutLineWidth = 0
    // eslint-disable-next-line no-labels
    outer: for (const line of input) {
      const chars = [...line]
      let nextWord: string[] = []
      let nextWordWidth = 0
      for (const char of chars) {
        const charWidth = Strings.width(char)
        if (wrap === 'word' && /^\w$/.test(char)) {
          // add to word
          // width will never be 0
          nextWord.push(char)
          for (let i = 1; i < charWidth; i++) {
            nextWord.push('')
          }
          nextWordWidth += charWidth
        } else {
          if (nextWord.length > 0) {
            // wrap line if necessary and add word
            if (nextOutLineWidth + nextWordWidth > width) {
              // nextWord.length > 0 implies wrap === 'word'
              // so wrap line
              if (result.length === height) {
                // no more room
                // eslint-disable-next-line no-labels
                break outer
              }
              result.push(nextOutLine)
              nextOutLine = []
              nextOutLineWidth = 0
            }

            // add word
            nextOutLine.push(...nextWord)
            nextOutLineWidth += nextWordWidth
            nextWord = []
            nextWordWidth = 0
          }

          if (charWidth === 0) {
            // zero-width char, so we add it to the last character so it's outside on overlap
            nextOutLine[nextOutLine.length - 1] += char
          } else {
            // wrap if necessary and add char
            if (nextOutLineWidth + charWidth > width) {
              switch (wrap) {
                case 'word':
                case 'char':
                  if (result.length === height) {
                    // no more room
                    // eslint-disable-next-line no-labels
                    break outer
                  }
                  result.push(nextOutLine)
                  nextOutLine = []
                  nextOutLineWidth = 0
                  break
                case 'clip':
                  // This breaks out of the switch and contiues the for loop, avoiding nextOutLine.push(char); ...
                  // (don't think too hard about it)
                  continue
                case undefined:
                  console.warn('text extended past width but wrap is undefined')
                  break
              }
            }

            // add char
            nextOutLine.push(char)
            for (let i = 1; i < charWidth; i++) {
              nextOutLine.push('')
            }
            nextOutLineWidth += charWidth
          }
        }
      }

      // add line
      if (result.length === height) {
        // no more room
        // eslint-disable-next-line no-labels
        break
      }
      result.push(nextOutLine)
      nextOutLine = []
      nextOutLineWidth = 0
    }

    if (color !== null) {
      const rgbColor = Color.toRGB(color)
      const { openEscape, closeEscape } = chalk.rgb(rgbColor.red * 255, rgbColor.green * 255, rgbColor.blue * 255)
      const fg = CharColor('fg', openEscape, closeEscape)
      VRender.addColor(result, fg)
    }

    VRender.translate1(result, bounds)
    return result
  }

  protected override renderSolidColor (rect: Rectangle, columnSize: Size, color: Color): VRender {
    if (rect.width === 0 || rect.height === 0) {
      return []
    }

    const rgbColor = Color.toRGB(color)
    const { openEscape, closeEscape } = chalk.bgRgb(rgbColor.red * 255, rgbColor.green * 255, rgbColor.blue * 255)
    const bg = CharColor('bg', openEscape, closeEscape)

    const result: VRender = range(rect.height).map(() => Array(rect.width).fill(` ${bg}`))

    VRender.translate2(result, rect.left, rect.top)
    return result
  }

  protected override renderBorder (rect: Rectangle, columnSize: Size, color: Color | null, borderStyle: BorderStyle): VRender {
    if (rect.width === 0 || rect.height === 0) {
      return []
    }

    let fg: string
    if (color !== null) {
      const rgbColor = Color.toRGB(color)
      const { openEscape, closeEscape } = chalk.rgb(rgbColor.red * 255, rgbColor.green * 255, rgbColor.blue * 255)
      fg = CharColor('fg', openEscape, closeEscape)
    } else {
      fg = ''
    }

    const border = BorderStyle.ASCII[borderStyle]
    const result: VRender = range(rect.height).map(i => {
      if (i === 0) {
        if (border.topAlt !== undefined) {
          return [border.topLeft, ...range(rect.width - 2).map(i => i % 2 === 0 ? border.top : border.topAlt), border.topRight]
        } else {
          return [border.topLeft, ...Array(rect.width - 2).fill(border.top), border.topRight]
        }
      } else if (i === rect.height - 1) {
        if (border.bottomAlt !== undefined) {
          return [border.bottomLeft, ...range(rect.width - 2).map(i => i % 2 === 0 ? border.bottom : border.bottomAlt), border.bottomRight]
        } else {
          return [border.bottomLeft, ...Array(rect.width - 2).fill(border.bottom), border.bottomRight]
        }
      } else {
        if (border.leftAlt !== undefined && border.rightAlt !== undefined) {
          return [i % 2 === 0 ? border.left : border.leftAlt, ...Array(rect.width - 2).fill(TRANSPARENT), i % 2 === 0 ? border.right : border.rightAlt]
        } else {
          return [border.left, ...Array(rect.width - 2).fill(TRANSPARENT), border.right]
        }
      }
    }).map(row => row.map((char: string) => char === TRANSPARENT ? char : char + fg))

    VRender.translate2(result, rect.left, rect.top)
    return result
  }

  protected override renderImage (bounds: BoundingBox, columnSize: Size, src: string, node: VNode): { render: VRender, size: Size } {
    const [image, resolveCallback] = this.assets.getImage(src, bounds.width, bounds.height)
    if (image === undefined) {
      throw new Error(`Image should not ever be undefined: ${src}`)
    } else if (image === null) {
      resolveCallback(() => this.invalidate(node))
      return {
        render: this.renderText(bounds, columnSize, 'clip', Color('gray'), '...'),
        size: { width: '...'.length, height: 1 }
      }
    } else {
      // render = deepCopy(image)
      const render = image.map(row => [...row])
      VRender.translate1(render, bounds)

      return {
        render,
        size: {
          width: Math.max(0, ...image.map(line => line.length)),
          height: image.length
        }
      }
    }
  }

  protected override renderVectorImage (bounds: BoundingBox, columnSize: Size, src: string): { render: VRender, size: Size } {
    // Don't render these in terminal
    return {
      render: [],
      size: { width: 0, height: 0 }
    }
  }

  protected override renderPixi (bounds: BoundingBox, columnSize: Size, pixi: DisplayObject | 'terminal'): { render: VRender, size: Size | null } {
    if (pixi !== 'terminal') {
      throw new Error('pixi DisplayObject should be null in terminal')
    }
    // Nothing
    return {
      render: [],
      size: null
    }
  }

  override useInput (handler: (key: Key) => void): () => void {
    function listener (keyStr: string, key: Key): void {
      if (key.name === undefined) {
        console.warn(`Unknown key: ${keyStr} ${JSON.stringify(key)}`)
      } else {
        handler(key)
      }
    }
    this.input.addListener('keypress', listener)
    return () => {
      this.input.removeListener('keypress', listener)
    }
  }

  override dispose (): void {
    super.dispose()
    this.interact.close()
  }
}
