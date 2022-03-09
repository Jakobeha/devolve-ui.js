import { TerminalInterface } from 'shims/terminal'
import { Key } from '@raycenity/misc-ts'

export function createTerminalInterface (): TerminalInterface {
  const terminalDiv = document.getElementById('terminal') ?? document.createElement('div')
  terminalDiv.id = 'terminal'
  terminalDiv.hidden = false
  let isPaused = false
  let cursorPosition: { x: number, y: number } | null = null
  const keypressListeners: Set<(str: string, key: Key) => void> = new Set()
  const overrideKeypress = (event: KeyboardEvent): void => {
    if (isPaused) return
    event.preventDefault()
    event.stopPropagation()
    for (const keypressListener of keypressListeners) {
      keypressListener(event.key, Key.fromKeyboardEvent(event))
    }
  }
  document.addEventListener('keydown', overrideKeypress)
  return {
    write: text => {
      terminalDiv.appendChild(document.createTextNode(text))
      cursorPosition = null
    },
    writeln: text => {
      terminalDiv.appendChild(document.createTextNode(text))
      terminalDiv.appendChild(document.createElement('br'))
      cursorPosition = null
    },
    question: async question => await new Promise(resolve => {
      terminalDiv.appendChild(document.createTextNode(question))
      const input = document.createElement('input')
      input.type = 'text'
      input.addEventListener('keydown', event => {
        if (event.key === 'Enter') {
          resolve(input.value)
          input.remove()
        }
        cursorPosition = null
      })
      terminalDiv.appendChild(input)
      cursorPosition = null
    }),
    pause (): void {
      isPaused = true
    },
    resume (): void {
      isPaused = false
    },
    on: (event: 'keypress', listener: (str: string, key: Key) => void): void => {
      keypressListeners.add(listener)
    },
    off: (event: 'keypress', listener: (str: string, key: Key) => void): void => {
      keypressListeners.delete(listener)
    },
    cursorTo: async (x: number, y?: number): Promise<void> => {
      cursorPosition = { x, y: y ?? terminalDiv.children.length }
    },
    moveCursor: async (dx: number, dy: number): Promise<void> => {
      if (cursorPosition === null) {
        cursorPosition = { x: terminalDiv.lastChild?.textContent?.length ?? 0, y: terminalDiv.children.length }
      }
      cursorPosition.x += dx
      cursorPosition.y += dy
    },
    clearScreen: async () => {
      for (const child of terminalDiv.children) {
        terminalDiv.removeChild(child)
      }
    },
    clearScreenDown: async () => {
      if (cursorPosition === null) {
        return
      }
      let y = 0
      for (const child of terminalDiv.children) {
        if (y > cursorPosition.y) {
          terminalDiv.removeChild(child)
        } else if (y === cursorPosition.y) {
          if (child.textContent !== null && child.textContent !== '') {
            child.textContent = child.textContent.substring(cursorPosition.x)
          } else {
            terminalDiv.removeChild(child)
          }
        }
        y++
      }
    },
    close (): void {
      for (const child of terminalDiv.children) {
        terminalDiv.removeChild(child)
      }
      terminalDiv.hidden = true
      document.removeEventListener('keydown', overrideKeypress)
    }
  }
}
