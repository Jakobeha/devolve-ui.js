import { PLATFORM } from 'core'
import type { TerminalImage } from 'shims/terminal-image'

function throw_ (error: Error): never {
  throw error
}

let terminalImage: TerminalImage
/* eslint-disable no-useless-catch */
/* eslint-disable @typescript-eslint/restrict-template-expressions */
/* eslint-disable @typescript-eslint/no-var-requires */
try {
  terminalImage =
    PLATFORM === 'web'
      ? require('shims/terminal-image').terminalImage
      : PLATFORM === 'cli'
        ? require('terminal-image').default
        : throw_(new Error(`Unsupported platform: ${PLATFORM}`))
} catch (error) {
  // Try block is needed to suppress esbuild warning
  throw error
}
/* eslint-enable no-useless-catch */
/* eslint-enable @typescript-eslint/restrict-template-expressions */
/* eslint-enable @typescript-eslint/no-var-requires */

export default terminalImage
