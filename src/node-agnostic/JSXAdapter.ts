export interface JSXAdapter<NodeType> {
  effect: <T>(fn: (prev?: T) => T, init?: T) => void
  memo: <T>(fn: () => T, equal: boolean) => () => T
  createComponent: <T>(Comp: (props: T) => NodeType, props: T) => NodeType
  insert: <T>(parent: any, accessor: (() => T) | T, marker?: any | null) => NodeType
  spread: <T>(node: any, accessor: (() => T) | T, skipChildren?: Boolean) => void
  mergeProps: (...sources: unknown[]) => unknown
}
