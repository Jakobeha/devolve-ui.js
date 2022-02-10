import { TerminalInterface } from 'shims/terminal'
import { Key } from '@raycenity/misc-ts'

let readline: typeof import('readline')

export function initModule (imports: { readline: typeof import('readline') }): void {
  readline = imports.readline
}

export function createTerminalInterface (): TerminalInterface {
  // Allow keypress events
  process.stdin.setRawMode(true)
  readline.emitKeypressEvents(process.stdin)

  // Create readline interface
  const readlineInterface = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
    terminal: true
  })

  return {
    write: process.stdout.write.bind(process.stdout),
    question: async question => await new Promise(resolve => readlineInterface.question(question, resolve)),
    pause () {
      readlineInterface.pause()
    },
    resume () {
      readlineInterface.resume()
    },
    on: (event: 'keypress', listener: (str: string, key: Key) => void): void => {
      process.stdin.on('keypress', listener)
    },
    off: (event: 'keypress', listener: (str: string, key: Key) => void): void => {
      process.stdin.removeListener('keypress', listener)
    },
    cursorTo: async (x: number, y?: number): Promise<void> => {
      return await new Promise(resolve => {
        readline.cursorTo(process.stdout, x, y, resolve)
      })
    },
    moveCursor: async (dx: number, dy: number): Promise<void> => {
      return await new Promise(resolve => {
        readline.moveCursor(process.stdout, dx, dy, resolve)
      })
    },
    clearScreenDown: async (): Promise<void> => {
      return await new Promise(resolve => {
        readline.clearScreenDown(process.stdout, resolve)
      })
    },
    clearScreen: async (): Promise<void> => {
      return await new Promise(resolve => {
        readline.cursorTo(process.stdout, 0, 0, () => {
          readline.clearScreenDown(process.stdout, resolve)
        })
      })
    },
    close (): void {
      readlineInterface.close()
    }
  }
}
