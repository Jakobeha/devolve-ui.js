import { createComponent, effect, insert, memo, mergeProps, render, spread } from 'solid-js/web'
import { JSX } from 'solid-js'
import { Renderer } from 'universal'

export * from 'universal'
export { JSX }

export const renderer: Renderer<JSX.Element> = {
  effect,
  memo,
  createComponent,
  insert,
  spread,
  mergeProps
}

export {
  render,
  effect,
  memo,
  createComponent,
  insert,
  spread,
  mergeProps
}
