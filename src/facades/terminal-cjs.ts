import { PLATFORM } from 'core'
import type { TerminalInterface } from 'shims/terminal'

function throw_ (error: Error): never {
  throw error
}

export let createTerminalInterface: () => TerminalInterface
/* eslint-disable no-useless-catch */
/* eslint-disable @typescript-eslint/restrict-template-expressions */
/* eslint-disable @typescript-eslint/no-var-requires */
try {
  createTerminalInterface =
    PLATFORM === 'web'
      ? require('shims/terminal-web').createTerminalInterface
      : PLATFORM === 'cli'
        ? require('shims/terminal-cli').createTerminalInterface
        : throw_(new Error(`Unsupported platform: ${PLATFORM}`))
} catch (error) {
  // Try block is needed to suppress esbuild warning
  throw error
}
/* eslint-enable no-useless-catch */
/* eslint-enable @typescript-eslint/restrict-template-expressions */
/* eslint-enable @typescript-eslint/no-var-requires */

export type { TerminalInterface }
