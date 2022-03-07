import type { Interface } from 'readline'
import type { ReadStream, WriteStream } from 'tty'
import { BorderStyle, BoundingBox, Color, Rectangle, Size, VNode } from 'core/vdom'
import { CoreRenderOptions } from 'core/renderer'
import { Key, range, Strings } from '@raycenity/misc-ts'
import { terminalImage } from '@raycenity/terminal-image-min'
import { CoreAssetCacher, RendererImpl, VRenderBatch } from 'renderer/common'
import { chalk } from '@raycenity/chalk-cross'

let readline: typeof import('readline')

export function initModule (imports: { readline: typeof import('readline') }): void {
  readline = imports.readline
}

/**
 * Each x/y index represents the character at that exact position in the terminal.
 * If the character is multi-width, then the next character will be empty.
 * If the character is \u{FFF0} it is transparent (the character under will be used).
 * If the character contains \u{FFF1} and \u{FFF2}, it is a background (characters above will also have the background unless they also contain a background)
 */
type VRender = string[][]

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
      const fg = CharColor(openEscape, closeEscape)
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
    const bg = CharColor(openEscape, closeEscape)

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
      fg = CharColor(openEscape, closeEscape)
    } else {
      fg = ''
    }

    const border = BorderStyle.ASCII[borderStyle]
    const result: VRender = range(rect.height).map(i => (
      i === 0
        ? [border.topLeft, ...Array(rect.width - 2).fill(border.top), border.topRight]
        : i === rect.height - 1
          ? [border.bottomLeft, ...Array(rect.width - 2).fill(border.bottom), border.bottomRight]
          : [border.left, ...Array(rect.width - 2).fill(TRANSPARENT), border.right]
    ).map((char: string) => char === TRANSPARENT ? char : char + fg))

    VRender.translate2(result, rect.left, rect.top)
    return result
  }

  protected override renderImage (bounds: BoundingBox, columnSize: Size, src: string, node: VNode): { render: VRender, size: Size } {
    const [image, resolveCallback] = this.assets.getImage(src, bounds.width, bounds.height)
    if (image === undefined) {
      throw new Error(`Could not get image for some unknown reason: ${src}`)
    } else if (image === null) {
      resolveCallback(() => this.invalidate(node))
      return {
        render: this.renderText(bounds, columnSize, 'clip', Color('gray'), '...'),
        size: { width: '...'.length, height: 1 }
      }
    } else {
      // TODO: Fix so color is rendered correctly instead of using renderText
      return {
        render: this.renderText(bounds, columnSize, 'clip', null, image),
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

module VRender {
  export function addColor (vrender: VRender, color: CharColor): void {
    for (const line of vrender) {
      for (let x = 0; x < line.length; x++) {
        line[x] += color
      }
    }
  }

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
        for (let x = 0; x < xOffset; x++) {
          line.unshift(TRANSPARENT)
        }
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
    const matrixSorted = Object.entries(textMatrix).sort(([lhs], [rhs]) => Number(rhs) - Number(lhs)).map(([, lines]) => lines)

    const result: string[][] = Array(height).fill(null).map(() => Array(length).fill(TRANSPARENT))
    for (const lines of matrixSorted) {
      for (let y = 0; y < lines.length; y++) {
        const line = lines[y]
        const resultLine = result[y]
        for (let x = 0; x < line.length; x++) {
          const resultChar = resultLine[x]
          const char = line[x]
          if (resultChar === TRANSPARENT) {
            // fall through
            resultLine[x] = char
          } else if (!CharColor.has(resultChar) && CharColor.has(char)) {
            // add background
            resultLine[x] += CharColor.get(char)!
          }
        }
      }
    }
    for (let y = 0; y < result.length; y++) {
      const line = result[y]
      let prevBg: CharColor | null = null
      for (let x = 0; x < line.length; x++) {
        const char = line[x]

        // Fill if fallthrough
        if (char === TRANSPARENT) {
          line[x] = ' '
        }

        // Add open or close for background
        const bg = CharColor.get(char)
        if (bg !== null) {
          line[x] = CharColor.remove(line[x])
        }
        if (prevBg !== bg) {
          if (bg !== null) {
            line[x] = CharColor.open(bg) + line[x]
          }
          if (prevBg !== null) {
            line[x] = CharColor.close(prevBg) + line[x]
          }
        }
        prevBg = bg
      }

      if (prevBg !== null) {
        line[line.length - 1] += CharColor.close(prevBg)
      }
    }
    return result
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

const TRANSPARENT = '\u{FFF0}'

type CharColor = string

function CharColor (openEscape: string, closeEscape: string): string {
  return `\u{FFF1}${openEscape}\u{FFF2}${closeEscape}`
}

module CharColor {
  export function has (string: string): boolean {
    return string.includes('\u{FFF1}')
  }

  export function get (string: string): CharColor | null {
    if (!string.includes('\u{FFF1}')) {
      return null
    } else {
      return string.substring(string.indexOf('\u{FFF1}'))
    }
  }

  export function remove (string: string): string {
    if (!string.includes('\u{FFF1}')) {
      return string
    } else {
      return string.substring(0, string.indexOf('\u{FFF1}'))
    }
  }

  export function open (bg: CharColor): string {
    return bg.substring(bg.indexOf('\u{FFF1}') + 1, bg.indexOf('\u{FFF2}'))
  }

  export function close (bg: CharColor): string {
    return bg.substring(bg.indexOf('\u{FFF2}') + 1)
  }
}
