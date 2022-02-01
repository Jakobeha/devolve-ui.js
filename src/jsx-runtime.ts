// noinspection JSUnusedLocalSymbols

import { VJSX } from 'core'
import { BoxAttrs, ImageAttrs, MatchCase, TextAttrs } from 'node-agnostic'

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
    for: {
      each: readonly any[] | undefined | null
      fallback?: VJSX
    }
    show: {
      when: any
      fallback?: VJSX
    }
    switch: {
      fallback?: VJSX
    }
    match: MatchCase<VJSX, any>
    errorBoundary: {
      fallback: VJSX | ((err: any) => VJSX)
    }
  }
}
/* eslint-enable @typescript-eslint/no-empty-interface */
/* eslint-enable @typescript-eslint/no-unused-vars */
