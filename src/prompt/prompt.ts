import { FirstParameter } from '@raycenity/misc-ts'

export class PromptCancelledError extends Error {
  constructor (message: string) {
    super(`prompt cancelled: ${message}`)
  }
}

export class PromptTimeoutError extends PromptCancelledError {
  constructor () {
    super('timeout')
  }
}

export class PromptReplacedError extends PromptCancelledError {
  constructor () {
    super('replaced by another prompt')
  }
}

export interface PromptSpec<Resolve = any> {
  resolve: (arg: Resolve) => void
  reject?: (arg: any) => void
}

export type PromptArgs<T extends PromptSpec | undefined> = Omit<T, 'resolve' | 'reject'>

export type PromptReturn<T extends PromptSpec | undefined> =
  Promise<T extends PromptSpec ? FirstParameter<T['resolve']> : never>
