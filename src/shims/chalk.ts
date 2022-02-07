import { ChalkInstance } from 'chalk'

export const chalk: ChalkInstance = new Proxy({}, {
  get: (target, name) => {
    throw new Error('TODO chalk shim for pixi.js')
  }
}) as any
