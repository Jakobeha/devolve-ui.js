import type { Interface } from 'readline'
import type { ReadStream, WriteStream } from 'tty'
import { BoundingBox, Color, Rectangle, Size, VNode } from 'core/vdom'
import { CoreRenderOptions } from 'core/renderer'
import { Key, Strings } from '@raycenity/misc-ts'
import { terminalImage } from '@raycenity/terminal-image-min'
import { CoreAssetCacher, RendererImpl, VRenderBatch } from 'renderer/common'
import { chalk } from '@raycenity/chalk-cross'

let readline: typeof import('readline')

export function initModule (imports: { readline: typeof import('readline') }): void {
  readline = imports.readline
}

type VRender = Array<Array<string | null>>

export interface TerminalRenderOptions extends CoreRenderOptions {
  input?: ReadStream
  output?: WriteStream
  interact?: Interface
}

class AssetCacher extends CoreAssetCacher {
  static async image (path: string, width?: number, height?: number): Promise<string[]> {
    try {
      return (await terminalImage.file(path, { width, height })).split('\n')
    } catch (exception) {
      // @ts-expect-error
      if (exception.code !== 'ENOENT') {
        throw exception
      }
      return ['?']
    }
  }

  getImage (path: string, width?: number, height?: number): [string[] | null, (didResolve: () => void) => void] {
    return this.getAsync(
      `${path}?width=${width ?? 'auto'}&height=${height ?? 'auto'}`,
      async path => await AssetCacher.image(path, width, height)
    )
  }
}

export class TerminalRendererImpl extends RendererImpl<VRender, AssetCacher> {
  private readonly interact: Interface
  private readonly input: ReadStream
  private readonly output: WriteStream

  private linesOutput: number = 0

  constructor (root: () => VNode, opts: TerminalRenderOptions = {}) {
    super(new AssetCacher(), opts)

    let { input, output, interact } = opts

    input = input ?? process.stdin
    output = output ?? process.stdout
    interact = interact ?? readline.createInterface({ input, output, terminal: true })

    this.interact = interact
    this.input = input
    this.output = output

    this.input.setRawMode(true)
    this.input.setEncoding('utf8')
    readline.emitKeypressEvents(this.input)

    this.finishInit(root)
  }

  protected override clear (): void {
    if (this.linesOutput !== 0) {
      this.output.moveCursor(0, -this.linesOutput)
      this.output.clearScreenDown()
      this.linesOutput = 0
    }
  }

  protected override writeRender (render: VRenderBatch<VRender>): void {
    const lines = VRender.collapse(render)
    for (const line of lines) {
      for (const char of line) {
        this.output.write(char)
      }
      this.output.write('\n')
    }
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

  protected override renderText (bounds: BoundingBox, wrap: 'word' | 'char' | 'clip' | undefined, text: string | string[]): VRender {
    const width = bounds.width ?? Infinity
    const height = bounds.height ?? Infinity
    const input = Array.isArray(text) ? text : text.split('\n')

    const result: VRender = []
    // all lines start with an empty character, for zero-width characters to be outside on overlap
    let nextOutLine: string[] = ['']
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

    VRender.translate1(result, bounds)
    return result
  }

  protected override renderSolidColor (rect: Rectangle, columnSize: Size, color: Color): VRender {
    const rgbColor = Color.toRGB(color)
    const { openEscape, closeEscape } = chalk.bgRgb(rgbColor.red, rgbColor.green, rgbColor.blue)
    const result: VRender = []
    // all lines start with an empty character, for zero-width characters to be outside on overlap
    let nextLine: string[] = []
    for (let y = 0; y < rect.height; y++) {
      nextLine.push(openEscape)
      for (let x = 0; x < rect.width; x++) {
        nextLine.push(' ')
      }
      nextLine.push(closeEscape)

      result.push(nextLine)
      nextLine = []
    }

    VRender.translate2(result, rect.left, rect.top)
    return result
  }

  protected override renderImage (bounds: BoundingBox, src: string, node: VNode): VRender {
    const [image, resolveCallback] = this.assets.getImage(src, bounds.width, bounds.height)
    if (image === undefined) {
      throw new Error(`Could not get image for some unknown reason: ${src}`)
    } else if (image === null) {
      resolveCallback(() => this.setNeedsRerender(node))
      return this.renderText(bounds, 'clip', '...')
    } else {
      return this.renderText(bounds, 'clip', image)
    }
  }

  protected override renderVectorImage (bounds: BoundingBox, src: string): VRender {
    // Don't render these in terminal
    return []
  }

  override useInput (handler: (key: Key) => void): () => void {
    function listener (chunk: string | Buffer): void {
      if (chunk instanceof Buffer) {
        chunk = chunk.toString()
      }
      for (const key of chunk) {
        handler({
          name: key,
          shift: key === key.toUpperCase(),
          ctrl: false,
          meta: false
        })
      }
    }
    function listener2 (keyStr: string, key: Key): void {
      if (key.name !== undefined) {
        // key.name is undefined on data input
        handler(key)
      }
    }
    this.input.addListener('data', listener)
    this.input.addListener('keypress', listener2)
    return () => {
      this.input.removeListener('keypress', listener2)
      this.input.removeListener('data', listener)
    }
  }

  override dispose (): void {
    super.dispose()
    this.interact.close()
  }
}

module VRender {
  export function translate1 (vrender: VRender, bounds: BoundingBox): void {
    const width = bounds.width ?? getWidth(vrender)
    const height = bounds.height ?? getHeight(vrender)

    const xOffset = bounds.x + (bounds.anchorX * width)
    const yOffset = bounds.y + (bounds.anchorY * height)

    return translate2(vrender, xOffset, yOffset)
  }

  export function translate2 (vrender: VRender, xOffset: number, yOffset: number): void {
    xOffset = Math.round(xOffset)
    yOffset = Math.round(yOffset)

    for (const line of vrender) {
      if (line.length > 0) {
        if (line[0] === '') {
          line[0] = null
        } else {
          line[0] = ' ' + (line[0] as string)
        }
        for (let x = 1; x < xOffset; x++) {
          line.unshift(null)
        }
        line.unshift('')
      }
    }
    for (let y = 0; y < yOffset; y++) {
      vrender.unshift([])
    }
  }

  export function collapse (textMatrix: Record<number, VRender>): string[][] {
    for (const key of Object.keys(textMatrix)) {
      if (isNaN(parseFloat(key))) {
        delete textMatrix[key as any]
      }
    }

    if (Object.values(textMatrix).length === 0) {
      return []
    }

    // Array length not width
    const length = Math.max(...Object.values(textMatrix).map(get2dArrayLength))
    const height = Math.max(...Object.values(textMatrix).map(getHeight))
    const matrixSorted = Object.entries(textMatrix).sort(([lhs], [rhs]) => Number(lhs) - Number(rhs)).map(([, lines]) => lines)

    const result: Array<Array<string | null>> = Array(height).fill(null).map(() => Array(length).fill(null))
    for (const lines of matrixSorted) {
      for (let y = 0; y < lines.length; y++) {
        const line = lines[y]
        const resultLine = result[y]
        for (let x = 0; x < line.length; x++) {
          if (resultLine[x] === null) {
            resultLine[x] = line[x]
          }
        }
      }
    }
    for (let y = 0; y < result.length; y++) {
      const line = result[y]
      for (let x = 0; x < line.length; x++) {
        if (line[x] === null) {
          line[x] = ' '
        }
      }
    }
    return result as string[][]
  }

  function getWidth (vrender: VRender): number {
    return Math.max(0, ...vrender.map(line => line.map(char => char === null ? 1 : Strings.width(char)).reduce((lhs, rhs) => lhs + rhs, 0)))
  }

  function get2dArrayLength (vrender: VRender): number {
    return Math.max(0, ...vrender.map(line => line.length))
  }

  function getHeight (vrender: VRender): number {
    return vrender.length
  }
}
