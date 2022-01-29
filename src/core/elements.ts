import { VBox, VImage, VJSX, VNode, VText } from 'core/vdom'
import { BoxAttrs, Elements, ImageAttrs, MatchCase } from 'node-agnostic'

export const elements: Elements<VJSX, VNode> = {
  Text: (props: { children: string[] }): VNode => VText(props.children.join('')),
  Box: (props: { children: VJSX } & BoxAttrs): VNode => {
    let children = VJSX.collapse(props.children)
    delete props.children

    return VBox(children, props)
  },
  Image: (props: { path: string } & ImageAttrs): VNode => {
    const path = props.path
    // @ts-expect-error
    delete props.path

    return VImage(path, props)
  },
  For: <T>({ each, fallback, children }: {
    each: readonly T[] | undefined | null | false
    fallback?: VJSX
    children: (item: T, index: number) => VJSX
  }): VJSX => {
    if (each === undefined || each === null || each === false || each.length === 0) {
      return fallback
    } else {
      return VJSX.collapse(each.map((item, index) => children(item, index)))
    }
  },
  Show: <T>({ when, fallback, children }: {
    when: T | undefined | null | false
    fallback?: VJSX
    children: VJSX | ((item: NonNullable<T>) => VJSX)
  }): VJSX => {
    if (when === undefined || when === null || when === false) {
      return fallback
    } else {
      return VJSX.collapse(typeof children === 'function' ? children(when as NonNullable<T>) : children)
    }
  },
  Switch: (props: {
    fallback?: VJSX
    children: MatchCase<VJSX, any> | MatchCase<VJSX, any>[]
  }): VJSX => {
    const cases = Array.isArray(props.children) ? props.children : [props.children]
    const fallback = props.fallback

    for (const case_ of cases) {
      if (case_.when !== undefined && case_.when !== null && case_.when !== false) {
        return VJSX.collapse(typeof case_.children === 'function' ? case_.children(case_.when) : case_.children)
      }
    }
    return fallback
  },
  Match: <T>(props: MatchCase<VJSX, T>): MatchCase<VJSX, T> => props,
  ErrorBoundary: ({ fallback, children }: {
    fallback: VJSX | ((err: any) => VJSX)
    children: VJSX
  }): VJSX => {
    try {
      return VJSX.collapse(children)
    } catch (err) {
      return typeof fallback === 'function' ? fallback(err) : fallback
    }
  }
}

export const {
  Box,
  Image,
  For,
  Show,
  Switch,
  Match,
  ErrorBoundary
} = elements

export function HBox(props: { children: VJSX } & Omit<BoxAttrs, 'direction'>): VNode {
  return Box( { ...props, direction: 'horizontal' })
}

export function YBox(props: { children: VJSX } & Omit<BoxAttrs, 'direction'>): VNode {
  return Box( { ...props, direction: 'vertical' })
}
