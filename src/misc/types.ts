export type AnyRecord = Record<string, unknown>
export type Empty = Record<string, never>

export type DataRecord = Record<string, Data>
export type LogicRecord = Record<string, Logic>

/** Not a function */
export type Data = string | number | boolean | null | undefined | Data[] | object

/** A pure function */
export type Logic = /* @__PURE__ */ (...args: any[]) => any
// I don't think the pure annotation actually does anything

export type NullToEmpty<T> = T extends null | undefined ? Empty : T

export type FirstArgumentType<T extends (arg: any, ...rest: any) => any> =
  T extends (arg: infer A, ...rest: any) => any ? A : never

export type ElemType<T extends any[]> = T extends Array<infer E> ? E : never

export type RecursiveElemType<T> = T extends Array<infer E> ? RecursiveElemType<E> : T

export type Unsubscribe = () => void
