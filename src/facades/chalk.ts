import { PLATFORM } from 'core'
import { ChalkInstance } from 'chalk'

/* eslint-disable @typescript-eslint/restrict-template-expressions */
export const chalk: ChalkInstance = await (
  PLATFORM === 'web'
    ? import('shims/chalk').then(m => m.chalk)
    : PLATFORM === 'cli'
      ? import('chalk').then(m => m.default)
      : Promise.reject(new Error(`Unsupported platform: ${PLATFORM}`))
)
/* eslint-enable @typescript-eslint/restrict-template-expressions */

export type { ForegroundColor, BackgroundColor, Color, Modifiers, Options, ChalkInstance } from 'chalk'
