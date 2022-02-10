import type { Interface } from 'readline'
import type { ReadStream, WriteStream } from 'tty'
import { VImage, VNode } from 'core/vdom'
import stringWidth from 'string-width'
import { CoreRenderOptions } from 'core/renderer'
import { Key, Strings } from '@raycenity/misc-ts'
import { terminalImage } from '@raycenity/terminal-image-min'
import { CoreAssetCacher, RendererImpl } from 'renderer/common'
import overlay = Strings.overlay

let readline: typeof import('readline')

export function initModule (imports: { readline: typeof import('readline') }): void {
  readline = imports.readline
}

interface VRender {
  lines: string[]
  width: number
  height: number
}

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

  protected override writeRender (render: VRender): void {
    for (const line of render.lines) {
      this.output.write(line)
      this.output.write('\n')
    }
    this.linesOutput += render.lines.length
  }

  protected override renderNodeImpl (node: VNode): VRender {
    if (VNode.isText(node)) {
      return this.renderText(node.text)
    } else if (VNode.isBox(node)) {
      const {
        visible,
        direction,
        gap,
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
          lines: [],
          width: 0,
          height: 0
        }
      }

      // Render children
      const children = this.renderBoxChildren(node.children, direction, gap)
      const lines = children.lines

      // Add padding
      if (paddingLeft !== undefined) {
        for (let y = 0; y < lines.length; y++) {
          lines[y] = ' '.repeat(paddingLeft) + lines[y]
        }
      }
      if (paddingTop !== undefined) {
        for (let y = 0; y < paddingTop; y++) {
          lines.unshift(' '.repeat(children.width))
        }
      }
      if (paddingRight !== undefined) {
        for (let y = 0; y < lines.length; y++) {
          lines[y] = lines[y] + ' '.repeat(paddingRight)
        }
      }
      if (paddingBottom !== undefined) {
        for (let y = 0; y < paddingBottom; y++) {
          lines.push(' '.repeat(children.width))
        }
      }

      // Add empty space or clip to get correct size
      if (width !== undefined) {
        if (children.width > width) {
          for (let y = 0; y < children.height; y++) {
            let line = lines[y]
            while (stringWidth(line) > width) {
              line = line.substring(0, line.length - 1)
            }
            lines[y] = line
          }
        } else if (children.width < width) {
          resizeLines(lines, width)
        }
      }
      if (height !== undefined) {
        if (children.height > height) {
          lines.splice(height, children.height - height)
        } else {
          for (let y = children.height; y < height; y++) {
            lines.push(' '.repeat(width ?? children.width))
          }
        }
      }

      // Add margin
      if (marginRight !== undefined) {
        for (let y = 0; y < lines.length; y++) {
          lines[y] = lines[y] + ' '.repeat(marginRight)
        }
      }
      if (marginBottom !== undefined) {
        for (let y = 0; y < marginBottom; y++) {
          lines.push(' '.repeat(width ?? children.width))
        }
      }
      if (marginLeft !== undefined) {
        for (let y = 0; y < lines.length; y++) {
          lines[y] = ' '.repeat(marginLeft) + lines[y]
        }
      }
      if (marginTop !== undefined) {
        for (let y = 0; y < marginTop; y++) {
          lines.unshift(' '.repeat(width ?? children.width))
        }
      }

      return {
        lines,
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
          lines: [],
          width: 0,
          height: 0
        }
      }

      const image = this.renderImage(node)
      if (width !== undefined) {
        resizeLines(image.lines, width)
      }
      if (height !== undefined) {
        if (image.height > height) {
          image.lines.splice(height, image.height - height)
        } else {
          for (let y = image.height; y < height; y++) {
            image.lines.push(' '.repeat(image.width))
          }
        }
      }

      return {
        lines: image.lines,
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
    resizeLines(lines, width)
    return {
      lines,
      width,
      height: lines.length
    }
  }

  private renderBoxChildren (children: VNode[], renderDirection?: 'horizontal' | 'vertical' | null, gap?: number): VRender {
    if (renderDirection === 'vertical') {
      const lines: string[] = []
      let width = 0
      let height = 0
      let isFirst = true
      for (const child of children) {
        if (gap !== undefined && !isFirst) {
          for (let y = 0; y < gap; y++) {
            lines.push(' '.repeat(width))
          }
        }
        isFirst = false
        const render = this.renderNodeImpl(child)
        lines.push(...render.lines)
        width = Math.max(width, render.width)
        height += render.height
      }
      resizeLines(lines, width)
      return { lines, width, height }
    } else if (renderDirection === 'horizontal') {
      const lines: string[] = []
      let width = 0
      let height = 0
      let isFirst = true
      for (const child of children) {
        if (gap !== undefined && !isFirst) {
          width += gap
          for (let y = 0; y < lines.length; y++) {
            lines[y] = lines[y] + ' '.repeat(gap)
          }
        }
        isFirst = false
        const render = this.renderNodeImpl(child)
        while (lines.length < render.lines.length) {
          lines.push(' '.repeat(width))
        }
        for (let y = 0; y < render.lines.length; y++) {
          const line = lines[y]
          const renderedLine = render.lines[y]
          lines[y] = line + renderedLine
        }
        width += render.width
        height = Math.max(height, render.height)
        resizeLines(lines, width)
      }
      return { lines, width, height }
    } else {
      if (gap !== undefined) {
        throw new Error('Gap is not supported for overlay (default) direction')
      }
      const childRenders = children.map(child => this.renderNodeImpl(child))
      return {
        lines: overlay(...childRenders.map(render => render.lines)),
        width: Math.max(0, ...childRenders.map(render => render.width)),
        height: Math.max(0, ...childRenders.map(render => render.height))
      }
    }
  }

  private renderImage (node: VImage, width?: number, height?: number): VRender {
    const path = node.path
    const [image, resolveCallback] = this.assets.getImage(path, width, height)
    if (image === undefined) {
      throw new Error(`Image not found: ${path}`)
    } else if (image === null) {
      resolveCallback(() => this.setNeedsRerender(node))
      return this.renderText('Loading...')
    } else {
      return {
        lines: image,
        width: Math.max(...image.map(line => stringWidth(line))),
        height: image.length
      }
    }
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

function resizeLines (lines: string[], width: number): void {
  for (let y = 0; y < lines.length; y++) {
    const line = lines[y]
    const difference = width - stringWidth(line)
    if (difference > 0) {
      lines[y] = line + ' '.repeat(difference)
    }
  }
}
