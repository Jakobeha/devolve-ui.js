import { RendererImpl } from 'renderer/common'
import { VNode } from 'core/vdom'

export function VRoot<T extends VNode> (construct: () => T, renderer: RendererImpl<any, any>): T {
  VRENDERER = renderer
  try {
    return construct()
  } finally {
    VRENDERER = null
  }
}

export function VComponent<T extends VNode> (construct: () => T): T {
  const vcomponent: VComponent = {
    node: {},
    renderer: getVRenderer(),
    state: [],
    onChange: [],
    construct,
    isBeingConstructed: true,
    nextStateIndex: 0
  }
  VCOMPONENT_STACK.push(vcomponent)
  try {
    const constructed = construct()
    Object.assign(vcomponent.node, constructed)
    vcomponent.isBeingConstructed = false
    VComponent.runObservers(vcomponent)
    return constructed
  } catch (e) {
    VComponent.reset(vcomponent)
    throw e
  } finally {
    VCOMPONENT_STACK.pop()
  }
}

export function getVRenderer (): RendererImpl<any, any> {
  if (VRENDERER === null) {
    throw new Error('Current renderer is not set')
  }
  return VRENDERER
}

export function getVComponent (): VComponent {
  if (VCOMPONENT_STACK.length === 0) {
    throw new Error('Components are not set')
  }
  return VCOMPONENT_STACK[VCOMPONENT_STACK.length - 1]
}

export interface VComponent {
  readonly node: Partial<VNode>
  readonly renderer: RendererImpl<any, any>
  readonly state: any[]
  readonly onChange: Array<() => void | Promise<void>>
  readonly construct: () => VNode
  isBeingConstructed: boolean
  nextStateIndex: number
}

export module VComponent {
  export function reset (vcomponent: VComponent): void {
    for (const prop in vcomponent.node) {
      // @ts-expect-error
      // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
      delete vcomponent.node[prop]
    }
    vcomponent.state.splice(0, vcomponent.nextStateIndex)
    vcomponent.nextStateIndex = 0
    vcomponent.isBeingConstructed = true
  }

  export function update (vcomponent: VComponent): void {
    vcomponent.nextStateIndex = 0
    VNode.convertInto(vcomponent.node, vcomponent.construct())
    vcomponent.renderer.setNeedsRerender(vcomponent.node)
    runObservers(vcomponent)
  }

  export function runObservers (vcomponent: VComponent): void {
    for (const onChange of vcomponent.onChange) {
      void onChange()
    }
  }
}

let VRENDERER: RendererImpl<any, any> | null = null
const VCOMPONENT_STACK: VComponent[] = []
