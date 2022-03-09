import { Key } from '@raycenity/misc-ts'

export interface TerminalInterface {
  write: (text: string) => void
  writeln: (text: string) => void
  question: (text: string) => Promise<string>
  pause: () => void
  resume: () => void
  on: (event: 'keypress', listener: (str: string, key: Key) => void) => void
  off: (event: 'keypress', listener: (str: string, key: Key) => void) => void
  cursorTo: (x: number, y?: number) => Promise<void>
  moveCursor: (dx: number, dy: number) => Promise<void>
  clearScreenDown: () => Promise<void>
  clearScreen: () => Promise<void>
  close: () => void
}
