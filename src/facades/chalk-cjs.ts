import { PLATFORM } from 'core'
import { ChalkInstance } from 'chalk'

function throw_ (error: Error): never {
  throw error
}

let Chalk: ChalkInstance
/* eslint-disable no-useless-catch */
/* eslint-disable @typescript-eslint/restrict-template-expressions */
/* eslint-disable @typescript-eslint/no-var-requires */
try {
  Chalk =
    PLATFORM === 'web'
      ? require('shims/chalk').chalk
      : PLATFORM === 'cli'
        ? require('chalk').default
        : throw_(new Error(`Unsupported platform: ${PLATFORM}`))
} catch (error) {
  // Try block is needed to suppress esbuild warning
  throw error
}
/* eslint-enable no-useless-catch */
/* eslint-enable @typescript-eslint/restrict-template-expressions */
/* eslint-enable @typescript-eslint/no-var-requires */

export type { ForegroundColor, BackgroundColor, Color, Modifiers, Options, ChalkInstance } from 'chalk'
export default Chalk
