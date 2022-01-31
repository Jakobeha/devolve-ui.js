export interface PrimitiveAttrs {
  className?: string
  visible?: boolean
}

export interface BoxAttrs extends PrimitiveAttrs{
  direction?: 'horizontal' | 'vertical' | null
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

export interface ImageAttrs extends PrimitiveAttrs {
  width?: number
  height?: number
}

export interface TextAttrs extends PrimitiveAttrs {
  transform?: (text: string) => string
}

export interface Primitives<JSXType, NodeType extends JSXType = JSXType> {
  Text: (props: TextAttrs, ...children: string[]) => NodeType
  Box: (props: BoxAttrs, ...children: JSXType[]) => NodeType
  Image: (props: { path: string } & ImageAttrs) => NodeType
}

export interface ControlFlow<JSXType> {
  For: <T>(props: {
    each: readonly T[] | undefined | null
    fallback?: JSXType
  }, children: (item: T, index: number) => JSXType) => JSXType
  Show: <T>(props: {
    when: T | undefined | null | false
    fallback?: JSXType
  }, ...children: JSXType[] | [((item: NonNullable<T>) => JSXType)]) => JSXType
  Switch: (props: {
    fallback?: JSXType
  }, ...children: Array<MatchCase<JSXType, any>>) => JSXType
  Match: <T>(props: MatchCase<JSXType, T>) => MatchCase<JSXType, T>
  ErrorBoundary: (props: {
    fallback: JSXType | ((err: any) => JSXType)
  }, ...children: JSXType[]) => JSXType
}

export interface MatchCase<JSXType, T>{
  when: T | undefined | null | false
  children: JSXType | ((item: NonNullable<T>) => JSXType)
}

export type Elements<JSXType, NodeType extends JSXType = JSXType> =
  Primitives<JSXType, NodeType> & ControlFlow<JSXType>
