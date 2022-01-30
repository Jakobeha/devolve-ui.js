// noinspection JSUnusedLocalSymbols

import { VJSX } from 'core'
import { BoxAttrs, ImageAttrs, TextAttrs } from 'node-agnostic'

/* eslint-disable @typescript-eslint/no-unused-vars */
/* eslint-disable @typescript-eslint/no-empty-interface */
export namespace JSX {
  export type Element = VJSX
  export interface IntrinsicElements {
    hbox: Omit<BoxAttrs, 'direction'>
    vbox: Omit<BoxAttrs, 'direction'>
    box: BoxAttrs
    text: TextAttrs
    image: ImageAttrs
  }
}
/* eslint-enable @typescript-eslint/no-empty-interface */
/* eslint-enable @typescript-eslint/no-unused-vars */
