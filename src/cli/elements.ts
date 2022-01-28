import { VJSX, VNode, VText } from 'cli/vdom'
import { BoxAttrs, Elements, MatchCase, PrimitiveAttrs } from 'universal'

export const elements: Elements<VJSX, VNode> = {
  Text: (props: { children: VJSX } & PrimitiveAttrs): VNode => {
    let children = VJSX.collapse(props.children)
    delete props.children

    return {
      tag: 'span',
      props,
      children,
      parent: null,
      renderer: null
    }

  },
  Box: (props: { children: VJSX } & BoxAttrs): VNode => {
    let children = VJSX.collapse(props.children)
    delete props.children

    return {
      tag: 'div',
      props,
      children,
      parent: null,
      renderer: null
    }
  },
  Newline: (): VNode => VText('\n'),
  For: <T>({ each, fallback, children }: {
    each: readonly T[] | undefined | null
    fallback?: VJSX
    children: (item: T, index: () => number) => VJSX
  }): VJSX => {
    if (each === undefined || each === null) {
      return fallback
    } else {
      return () => {
        if (each.length === 0) {
          return fallback
        } else {
          return VJSX.collapse(each.map((item, index) => children(item, () => index)))
        }
      }
    }
  },
  Index: <T>({ each, fallback, children }: {
    each: readonly T[] | undefined | null
    fallback?: VJSX
    children: (item: () => T, index: number) => VJSX
  }): VJSX => {
    if (each === undefined || each === null) {
      return fallback
    } else {
      return () => {
        if (each.length === 0) {
          return fallback
        } else {
          return VJSX.collapse(each.map((item, index) => children(() => item, index)))
        }
      }
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
      return () => VJSX.collapse(
        typeof children === 'function' ? children(when as NonNullable<T>) : children
      )
    }
  },
  Switch: (props: {
    fallback?: VJSX
    children: MatchCase<VJSX, any> | MatchCase<VJSX, any>[]
  }): VJSX => {
    const cases = Array.isArray(props.children) ? props.children : [props.children]
    const fallback = props.fallback

    return () => {
      for (const case_ of cases) {
        if (case_.when !== undefined && case_.when !== null && case_.when !== false) {
          return VJSX.collapse(typeof case_.children === 'function' ? case_.children(case_.when) : case_.children)
        }
      }
      return fallback
    }
  },
  Match: <T>(props: MatchCase<VJSX, T>): MatchCase<VJSX, T> => props,
  ErrorBoundary: ({ fallback, children }: {
    fallback: VJSX | ((err: any, reset: () => void) => VJSX)
    children: VJSX
  }): VJSX => {
    return () => {
      try {
        return VJSX.collapse(children)
      } catch (err) {
        return typeof fallback === 'function' ? fallback(err, () => {}) : fallback
      }
    }
  }
}
