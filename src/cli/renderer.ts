import { createInterface, Interface } from 'readline'
import { ReadStream, WriteStream } from 'tty'
import * as process from 'process'
import { VNode } from 'universal/vdom'
import stringWidth from 'string-width'
import { CoreAssetCacher, CoreRenderOptions, RendererImpl } from 'universal/renderer'
import { Strings } from 'misc'
import terminalImage from 'terminal-image'
import overlay = Strings.overlay

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
  static async image(path: string, width?: number, height?: number): Promise<string[]> {
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

  getImage(path: string, width?: number, height?: number): [string[] | null, (didResolve: () => void) => void] {
    return this.getAsync(
      `${path}?width=${width}&height=${height}`,
        path => AssetCacher.image(path, width, height)
    )
  }
}

export class TerminalRendererImpl extends RendererImpl<VRender, AssetCacher> {
  private readonly interact: Interface
  private readonly output: WriteStream

  private linesOutput: number = 0

  constructor(opts: TerminalRenderOptions = {}) {
    super(new AssetCacher(), opts)
    let {input, output, interact} = opts

    input = input ?? process.stdin
    output = output ?? process.stdout
    interact = interact ?? createInterface({ input, output, terminal: true })

    this.interact = interact
    this.output = output
  }

  protected override clear() {
    if (this.linesOutput !== 0) {
      this.output.moveCursor(0, -this.linesOutput)
      this.output.clearScreenDown()
      this.linesOutput = 0
    }
  }

  protected override writeRender(render: VRender) {
    for (const line of render.lines) {
      this.output.write(line)
      this.output.write('\n')
    }
    this.linesOutput += render.lines.length
  }

  protected override renderNode(node: VNode): VRender {
    if (VNode.isText(node)) {
      return this.renderText(node.text)
    } else if (node.tag === 'box') {
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
      } = node.props
      if (!visible) {
        return {
          lines: [],
          width: 0,
          height: 0
        }
      }

      // Render children
      const children = this.renderBoxChildren(node.children, direction)
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
            const line = lines[y]
            const width = stringWidth(line)
            lines[y] = line.slice(0, width)
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
    } else if (node.tag === 'image') {
      const {
        visible,
        path,
        width,
        height,
      } = node.props
      if (!visible) {
        return {
          lines: [],
          width: 0,
          height: 0
        }
      }

      const image = this.renderImage(path ?? '')
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
      throw new Error(`Unhandled tag: ${node.tag}`)
    }
  }

  private renderText(text: string): VRender {
    const lines = text.split('\n')
    const width = lines.reduce((max, line) => Math.max(max, stringWidth(line)), 0)
    resizeLines(lines, width)
    return {
      lines,
      width,
      height: lines.length
    }
  }

  private renderBoxChildren(children: VNode[], renderDirection?: 'horizontal' | 'vertical' | null): VRender {
    if (renderDirection === 'vertical') {
      const lines: Array<string> = []
      let width = 0
      let height = 0
      for (const child of children) {
        const render = this.renderNode(child)
        lines.push(...render.lines)
        width = Math.max(width, render.width)
        height += render.height
      }
      resizeLines(lines, width)
      return {lines, width, height}
    } else if (renderDirection === 'horizontal') {
      const lines: Array<string> = []
      let width = 0
      let height = 0
      for (const child of children) {
        const render = this.renderNode(child)
        while (lines.length < render.lines.length) {
          lines.push(' '.repeat(width))
        }
        for (let y = 0; y < render.lines.length; y++) {
          let line = lines[y]
          const renderedLine = render.lines[y]
          line += renderedLine
        }
        width += render.width
        height = Math.max(height, render.height)
        resizeLines(lines, width)
      }
      return {lines, width, height}
    } else {
      const childRenders = children.map(child => this.renderNode(child))
      return {
        lines: overlay(...childRenders.map(render => render.lines)),
        width: Math.max(...childRenders.map(render => render.width)),
        height: Math.max(...childRenders.map(render => render.height))
      }
    }
  }

  private renderImage(path: string, width?: number, height?: number): VRender {
    const [image, resolveCallback] = this.assets.getImage(path, width, height)
    if (image === undefined) {
      throw new Error(`Image not found: ${path}`)
    } else if (image === null) {
      resolveCallback(() => this.setNeedsRerender())
      return this.renderText('Loading...')
    } else {
      return {
        lines: image,
        width: Math.max(...image.map(stringWidth)),
        height: image.length
      }
    }
  }

  override dispose() {
    super.dispose()
    this.interact.close()
  }
}

function resizeLines(lines: string[], width: number) {
  for (let y = 0; y < lines.length; y++) {
    const line = lines[y]
    const difference = width - stringWidth(line)
    if (difference > 0) {
      lines[y] = line + ' '.repeat(difference)
    }
  }
}
