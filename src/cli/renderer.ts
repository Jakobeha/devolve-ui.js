import { createInterface, Interface } from 'readline'
import { ReadStream, WriteStream } from 'tty'
import * as process from 'process'
import { VElement, VNode, VRoot } from 'cli/vdom'
import stringWidth from 'string-width'
import isText = VNode.isText

type Timer = NodeJS.Timer

interface VRender {
  lines: Array<string>
  width: number
  height: number
}

export class Renderer {
  static readonly DEFAULT_FPS: number = 20

  private readonly interact: Interface
  private readonly output: WriteStream
  private readonly _root: VElement = VRoot(this)

  private needsRerender: boolean = true
  private linesOutput: number = 0
  private timer: Timer | null = null

  get root(): VElement {
    return this._root
  }

  constructor(input?: ReadStream, output?: WriteStream, interact?: Interface) {
    input = input ?? process.stdin
    output = output ?? process.stdout
    interact = interact ?? createInterface({ input, output, terminal: true })

    this.interact = interact
    this.output = output
  }

  start(fps?: number) {
    if (this.timer !== null) {
      throw new Error('Renderer is already running')
    }

    this.timer = setInterval(() => {
      if (this.needsRerender) {
        this.rerender()
      }
    }, 1 / (fps ?? Renderer.DEFAULT_FPS))
  }

  stop() {
    if (this.timer === null) {
      throw new Error('Renderer is not running')
    }

    clearInterval(this.timer)
    this.timer = null
  }

  setNeedsRerender() {
    this.needsRerender = true
  }

  private rerender() {
    if (!this.needsRerender) return

    this.needsRerender = false
    this.clear()
    this.render()
  }

  private clear() {
    if (this.linesOutput !== 0) {
      this.output.moveCursor(0, -this.linesOutput)
      this.output.clearScreenDown()
      this.linesOutput = 0
    }
  }

  private render() {
    this.writeRender(this.renderNode(this.root))
  }

  private writeRender(render: VRender) {
    for (const line of render.lines) {
      this.output.write(line)
      this.output.write('\n')
    }
    this.linesOutput += render.lines.length
  }

  private renderNode(node: VNode): VRender {
    if (isText(node)) {
      const lines = node.text.split('\n')
      const width = lines.reduce((max, line) => Math.max(max, stringWidth(line)), 0)
      resizeLines(lines, width)
      return {
        lines,
        width,
        height: lines.length
      }
    } else {
      const {
        display,
        flexDirection,
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
      if (display === 'none') {
        return {
          lines: [],
          width: 0,
          height: 0
        }
      }

      // Render children
      const children = this.renderChildren(node.children, flexDirection === 'column')
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
        width: width ?? children.width,
        height: height ?? children.height
      }
    }
  }

  private renderChildren(children: VNode[], renderColumn: boolean): VRender {
    if (renderColumn) {
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
    } else {
      const lines: Array<string> = []
      let width = 0
      let height = 0
      for (const child of children) {
        const render = this.renderNode(child)
        while (lines.length < render.lines.length) {
          lines.push(''.repeat(width))
        }
        for (let y = 0; y < render.lines.length; y++) {
          let line = lines[y]
          const renderedLine = render.lines[y]
          line += renderedLine
        }
        lines.push(...render.lines)
        width += render.width
        height = Math.max(height, render.height)
        resizeLines(lines, width)
      }
      return {lines, width, height}
    }
  }

  dispose() {
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
