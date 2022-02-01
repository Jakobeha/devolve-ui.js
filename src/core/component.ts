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
    onDestroy: [],
    children: [],
    construct,
    isBeingConstructed: true,
    updateRightAfterConstruct: false,
    nextStateIndex: 0
  }

  if (VCOMPONENT_STACK.length === 0) {
    if (VRENDERER!.rootComponent !== null) {
      throw new Error('there can only be one root component')
    }
    VRENDERER!.rootComponent = vcomponent
  } else {
    const parent = VCOMPONENT_STACK[VCOMPONENT_STACK.length - 1]
    parent.children.push(vcomponent)
  }

  VCOMPONENT_STACK.push(vcomponent)
  try {
    const constructed = construct()
    // noinspection SuspiciousTypeOfGuard
    if (constructed === undefined || constructed === null || typeof constructed === 'function' || constructed instanceof Array) {
      // noinspection ExceptionCaughtLocallyJS
      throw new Error('JSX components can only be nodes (not fragments, functions, null, or undefined)')
    } else {
      Object.assign(vcomponent.node, constructed)
      vcomponent.isBeingConstructed = false
      if (vcomponent.updateRightAfterConstruct) {
        VComponent.update(vcomponent)
        vcomponent.updateRightAfterConstruct = false
      } else {
        VComponent.runObservers(vcomponent)
      }
      return vcomponent.node as T
    }
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
  readonly onChange: Array<(() => void | Promise<void>) | null>
  readonly onDestroy: Array<(() => void | Promise<void>) | null>
  readonly children: VComponent[]
  readonly construct: () => VNode
  isBeingConstructed: boolean
  updateRightAfterConstruct: boolean
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
    if (vcomponent.isBeingConstructed) {
      vcomponent.updateRightAfterConstruct = true
    } else {
      runDestroys(vcomponent)
      vcomponent.nextStateIndex = 0

      const prevRenderer = VRENDERER
      VRENDERER = vcomponent.renderer
      VCOMPONENT_STACK.push(vcomponent)
      try {
        VNode.convertInto(vcomponent.node, vcomponent.construct())
      } finally {
        VCOMPONENT_STACK.pop()
        VRENDERER = prevRenderer
      }

      if (!vcomponent.updateRightAfterConstruct) {
        vcomponent.renderer.setNeedsRerender(vcomponent.node)
      }

      runObservers(vcomponent)
    }
  }

  export function runObservers (vcomponent: VComponent): void {
    for (const onChange of vcomponent.onChange) {
      void onChange?.()
    }
  }

  export function runDestroys (vcomponent: VComponent): void {
    for (const onDestroy of vcomponent.onDestroy) {
      void onDestroy?.()
    }
    vcomponent.onDestroy.splice(0, vcomponent.onDestroy.length)
    for (const child of vcomponent.children) {
      runDestroys(child)
    }
  }
}

let VRENDERER: RendererImpl<any, any> | null = null
const VCOMPONENT_STACK: VComponent[] = []
