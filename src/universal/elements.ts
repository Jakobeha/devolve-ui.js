import { JSX } from 'solid-js'

export interface PrimitiveAttrs {
  className?: string
  visible?: boolean
}

export interface TextAttrs extends PrimitiveAttrs {}

export interface BoxAttrs extends PrimitiveAttrs{
  direction?: 'row' | 'column'
  width?: number
  height?: number
  marginLeft?: number
  marginRight?: number
  marginTop?: number
  marginBottom?: number
  paddingLeft?: number
  paddingRight?: number
  paddingTop?: number
  paddingBottom?: number
}

export interface Primitives<JSXType, NodeType extends JSXType = JSXType> {
  Text: (props: { children: JSXType } & TextAttrs) => NodeType
  Box: (props: { children: JSXType } & BoxAttrs) => NodeType
  Newline: () => NodeType
}

export interface ControlFlow<JSXType> {
  For: <T>(props: {
    each: readonly T[] | undefined | null
    fallback?: JSXType
    children: (item: T, index: () => number) => JSXType
  }) => JSXType
  Index: <T>(props: {
    each: readonly T[] | undefined | null
    fallback?: JSXType
    children: (item: () => T, index: number) => JSXType
  }) => JSXType
  Show: <T>(props: {
    when: T | undefined | null | false
    fallback?: JSXType
    children: JSXType | ((item: NonNullable<T>) => JSXType)
  }) => JSXType
  Switch: (props: {
    fallback?: JSXType
    children: MatchCase<JSXType, any> | MatchCase<JSXType, any>[]
  }) => JSXType
  Match: <T>(props: MatchCase<JSXType, T>) => MatchCase<JSXType, T>
  ErrorBoundary: (props: {
    fallback: JSXType | ((err: any, reset: () => void) => JSXType)
    children: JSXType
  }) => JSXType
}

export interface MatchCase<JSXType, T>{
  when: T | undefined | null | false
  children: JSXType | ((item: NonNullable<T>) => JSXType)
}

export type Elements<JSXType, NodeType extends JSXType = JSXType> =
  Primitives<JSXType, NodeType> & ControlFlow<JSXType>
