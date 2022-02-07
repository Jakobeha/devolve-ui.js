import { PLATFORM } from 'core'
import type { TerminalInterface } from 'shims/terminal'

/* eslint-disable @typescript-eslint/restrict-template-expressions */
export const createTerminalInterface: () => TerminalInterface = await (
  PLATFORM === 'web'
    ? import('shims/terminal-web').then(m => m.createTerminalInterface)
    : PLATFORM === 'cli'
      ? import('shims/terminal-cli').then(m => m.createTerminalInterface)
      : Promise.reject(new Error(`Unsupported platform: ${PLATFORM}`))
)
/* eslint-enable @typescript-eslint/restrict-template-expressions */

export type { TerminalInterface }
