import { RendererImpl } from 'renderer/common'
import { PLATFORM } from 'core/platform'
import { PixiComponent, VNode } from 'core/vdom'

type PendingUpdateDetails = string

const MAX_RECURSIVE_UPDATES_BEFORE_LOOP_DETECTED = 100

export interface VComponent<Props = any> {
  readonly node: Partial<VNode>
  children: Record<string, VComponent>
  readonly renderer: RendererImpl<any, any>

  props: Props
  construct: (props: Props) => VNode
  readonly state: any[]
  readonly effects: Array<() => void>
  readonly updateDestructors: Array<() => void>
  nextUpdateDestructors: Array<() => void>
  readonly permanentDestructors: Array<() => void>

  isBeingCreated: boolean
  isBeingUpdated: boolean
  isFresh: boolean
  isDead: boolean
  hasPendingUpdates: boolean
  recursiveUpdateStackTrace: PendingUpdateDetails[]
  nextStateIndex: number
}

const RENDERER_STACK: Array<RendererImpl<any, any>> = []
const VCOMPONENT_STACK: VComponent[] = []
let IS_DEBUG_MODE: boolean = true

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

export function isDebugMode (): boolean {
  return IS_DEBUG_MODE
}

export function setDebugMode (debugMode: boolean): void {
  IS_DEBUG_MODE = debugMode
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
          VComponent.update(vcomponent, `child ${key}`)
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
      nextUpdateDestructors: [],
      permanentDestructors: [],

      isBeingCreated: true,
      isBeingUpdated: false,
      isFresh: true,
      isDead: false,
      hasPendingUpdates: false,
      recursiveUpdateStackTrace: [],
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

      const node: VNode = vcomponent.node as VNode
      if (node.type === 'pixi') {
        if (PLATFORM === 'web') {
          const pixiComponent: PixiComponent<any> = vcomponent.construct as PixiComponent<any>
          const PIXI = (globalThis as unknown as { PIXI: typeof import('pixi.js') }).PIXI
          node.pixi = pixiComponent.lifecycle.mkPixi(PIXI)
          pixiComponent.pixis.push(node.pixi)
        } else {
          node.pixi = 'terminal'
        }
      }
    })
    vcomponent.isBeingCreated = false
    return vcomponent.node as VNode
  }

  export function update (vcomponent: VComponent, details: PendingUpdateDetails | null): void {
    if (vcomponent.isBeingUpdated) {
      // Delay until after this update, especially if there are multiple triggered updates since we only have to update once more
      vcomponent.hasPendingUpdates = true
      if (isDebugMode() && details !== null) {
        vcomponent.recursiveUpdateStackTrace.push(details)
      }
    } else {
      // Reset
      runUpdateDestructors(vcomponent)
      vcomponent.nextStateIndex = 0

      // Do construct
      // We also need to use VComponent's renderer because the current renderer might be different
      withRenderer(vcomponent.renderer, () => doUpdate(vcomponent, () => {
        VNode.convertInto(vcomponent.node, vcomponent.construct(vcomponent.props))

        const node: VNode = vcomponent.node
        if (node.type === 'pixi' && node.pixi !== 'terminal') {
          const pixiComponent: PixiComponent<any> = vcomponent.construct as PixiComponent<any>
          pixiComponent.lifecycle.update?.(node.pixi)
        }
      }))
      vcomponent.renderer.invalidate(vcomponent.node as VNode)
    }
  }

  export function destroy (vcomponent: VComponent): void {
    if (vcomponent.isDead) {
      throw new Error('sanity check: tried to destroy already dead component')
    }

    vcomponent.renderer.invalidate(vcomponent.node as VNode)

    const node: VNode = vcomponent.node as VNode
    if (node.type === 'pixi' && node.pixi !== 'terminal') {
      const pixiComponent: PixiComponent<any> = vcomponent.construct as PixiComponent<any>
      pixiComponent.lifecycle.destroy?.(node.pixi)
      pixiComponent.pixis.splice(pixiComponent.pixis.indexOf(node.pixi), 1)
    }

    runPermanentDestructors(vcomponent)
    vcomponent.isDead = true

    for (const child of Object.values(vcomponent.children)) {
      // set parent to undefined before destroy so it doesn't invalidate the parent again
      child.node.parent = undefined
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
      if (vcomponent.recursiveUpdateStackTrace.length > MAX_RECURSIVE_UPDATES_BEFORE_LOOP_DETECTED) {
        throw new Error(`update loop detected:\n${vcomponent.recursiveUpdateStackTrace.map(details => JSON.stringify(details)).join('\n')}`)
      }
      update(vcomponent, null)
    } else if (isDebugMode()) {
      vcomponent.recursiveUpdateStackTrace = []
    }
  }

  function clearFreshAndRemoveStaleChildren (vcomponent: VComponent): void {
    for (const [childKey, child] of Object.entries(vcomponent.children)) {
      if (child.isFresh) {
        child.isFresh = false
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
    vcomponent.updateDestructors.push(...vcomponent.nextUpdateDestructors)
    vcomponent.nextUpdateDestructors = []
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
