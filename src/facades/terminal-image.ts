import { PLATFORM } from 'core'
import type { TerminalImage } from 'shims/terminal-image'

/* eslint-disable @typescript-eslint/restrict-template-expressions */
export const terminalImage: TerminalImage = await (
  PLATFORM === 'web'
    ? import('shims/terminal-image').then(m => m.terminalImage)
    : PLATFORM === 'cli'
      ? import('terminal-image').then(m => m.default)
      : Promise.reject(new Error(`Unsupported platform: ${PLATFORM}`))
)
/* eslint-enable @typescript-eslint/restrict-template-expressions */
