import { RendererImpl } from 'renderer/common'
import { VNode } from 'core/vdom'

export interface VComponent<Props = any> {
  readonly node: Partial<VNode>
  children: Record<string, VComponent>
  readonly renderer: RendererImpl<any, any>

  props: Props
  construct: (props: Props) => VNode
  readonly state: any[]
  readonly effects: Array<() => void>
  readonly updateDestructors: Array<() => void>
  readonly permanentDestructors: Array<() => void>

  isBeingCreated: boolean
  isBeingUpdated: boolean
  isFresh: boolean
  isDead: boolean
  hasPendingUpdates: boolean
  nextStateIndex: number
}

const RENDERER_STACK: Array<RendererImpl<any, any>> = []
const VCOMPONENT_STACK: VComponent[] = []

export function getRenderer (): RendererImpl<any, any> {
  if (RENDERER_STACK.length === 0) {
    throw new Error('No current renderer')
  }
  return RENDERER_STACK[RENDERER_STACK.length - 1]
}

export function getVComponent (): VComponent {
  if (VCOMPONENT_STACK.length === 0) {
    throw new Error('No current component')
  }
  return VCOMPONENT_STACK[VCOMPONENT_STACK.length - 1]
}

function withVComponent<T> (vcomponent: VComponent, body: () => T): T {
  VCOMPONENT_STACK.push(vcomponent)
  try {
    return body()
  } finally {
    VCOMPONENT_STACK.pop()
  }
}

function withRenderer<T> (vrenderer: RendererImpl<any, any>, body: () => T): T {
  RENDERER_STACK.push(vrenderer)
  try {
    return body()
  } finally {
    RENDERER_STACK.pop()
  }
}

export function VRoot<T extends VNode> (renderer: RendererImpl<any, any>, construct: () => T): T {
  const node = withRenderer(renderer, construct)
  node.parent = 'none'
  return node
}

export function VComponent<Props> (key: string, props: Props, construct: (props: Props) => VNode): VNode {
  if (VCOMPONENT_STACK.length !== 0) {
    const parent = getVComponent()
    // parent is being created = if there are any existing children, they're not being reused, they're a conflict
    if (!parent.isBeingCreated) {
      if (key in parent.children) {
        const vcomponent = parent.children[key]
        // If the componennt was already reused this update, it's a conflict. We fallthrough to newVComponent which throws the error
        if (!vcomponent.isFresh) {
          vcomponent.props = props
          vcomponent.construct = construct
          vcomponent.isFresh = true
          VComponent.update(vcomponent)
          return vcomponent.node as VNode
        }
      }
    }
  }

  return VComponent.create(key, props, construct)
}

export module VComponent {
  export function create<Props> (key: string, props: Props, construct: (props: Props) => VNode): VNode {
    // Create JS object
    const vcomponent: VComponent<Props> = {
      node: {},
      children: {},
      renderer: getRenderer(),

      props,
      construct,
      state: [],
      effects: [],
      updateDestructors: [],
      permanentDestructors: [],

      isBeingCreated: true,
      isBeingUpdated: false,
      isFresh: true,
      isDead: false,
      hasPendingUpdates: false,
      nextStateIndex: 0
    }

    // Set parent
    if (VCOMPONENT_STACK.length === 0) {
      const currentRenderer = getRenderer()
      if (currentRenderer.rootComponent !== null) {
        throw new Error('there can only be one root component')
      }
      currentRenderer.rootComponent = vcomponent
      vcomponent.isFresh = false
    } else {
      const parent = getVComponent()
      if (key in parent.children) {
        throw new Error(`multiple components with the same parent and key: ${key}. Please assign different keys so that devolve-ui can distinguish the components in updates`)
      }
      parent.children[key] = vcomponent
    }

    // Do construct (component and renderer are already set)
    doUpdate(vcomponent, () => {
      const constructed = construct(vcomponent.props)
      if (typeof constructed !== 'object' || Array.isArray(constructed)) {
        throw new Error('JSX components can only be nodes. Call this function normally, not with JSX')
      }
      Object.assign(vcomponent.node, constructed)
    })
    vcomponent.isBeingCreated = false
    return vcomponent.node as VNode
  }

  export function update (vcomponent: VComponent): void {
    if (vcomponent.isBeingUpdated) {
      // Delay until after this update, especially if there are multiple triggered updates since we only have to update once more
      vcomponent.hasPendingUpdates = true
    } else {
      // Reset
      runUpdateDestructors(vcomponent)
      vcomponent.nextStateIndex = 0

      // Do construct
      // We also need to use VComponent's renderer because the current renderer might be different
      withRenderer(vcomponent.renderer, () => doUpdate(vcomponent, () => {
        VNode.convertInto(vcomponent.node, vcomponent.construct(vcomponent.props))
      }))
    }
  }

  export function destroy (vcomponent: VComponent): void {
    if (vcomponent.isDead) {
      throw new Error('sanity check: tried to destroy already dead component')
    }

    runPermanentDestructors(vcomponent)
    vcomponent.isDead = true

    for (const child of Object.values(vcomponent.children)) {
      destroy(child)
    }
  }

  function doUpdate (vcomponent: VComponent, body: () => void): void {
    if (vcomponent.isDead) {
      throw new Error('sanity check: tried to update dead component')
    }

    withVComponent(vcomponent, () => {
      vcomponent.isBeingUpdated = true
      // This will update state, add events, etc.
      body()
      clearFreshAndRemoveStaleChildren(vcomponent)
      vcomponent.isBeingUpdated = false
      runEffects(vcomponent)
    })
    if (vcomponent.hasPendingUpdates) {
      vcomponent.hasPendingUpdates = false
      update(vcomponent)
    }
  }

  function clearFreshAndRemoveStaleChildren (vcomponent: VComponent): void {
    for (const [childKey, child] of Object.entries(vcomponent.children)) {
      if (child.isFresh) {
        child.isFresh = false
        clearFreshAndRemoveStaleChildren(child)
      } else {
        destroy(child)
        delete vcomponent.children[childKey]
      }
    }
  }

  function runEffects (vcomponent: VComponent): void {
    // Effects might add new effects
    // If there are pending updates, we don't want to run any effects, because they will be run in the pending update
    // Of course, effects can cause more pending updates
    while (vcomponent.effects.length > 0 && !vcomponent.hasPendingUpdates) {
      const effect = vcomponent.effects.pop()!
      effect()
    }
    // Child effects are taken care of
  }

  function runUpdateDestructors (vcomponent: VComponent): void {
    // Destructors might add new destructors
    while (vcomponent.updateDestructors.length > 0) {
      const destructor = vcomponent.updateDestructors.pop()!
      destructor()
    }
    // Child update (and permanent if necessary) destructors are taken care of
  }

  function runPermanentDestructors (vcomponent: VComponent): void {
    // Destructors might add new destructors
    while (vcomponent.permanentDestructors.length > 0) {
      const destructor = vcomponent.permanentDestructors.pop()!
      destructor()
    }
    // Child permanent destructors are taken care of
  }
}
