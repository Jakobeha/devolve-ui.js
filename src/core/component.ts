import { PLATFORM } from 'core/platform'
import { PixiComponent, VNode, VNodeNode } from 'core/vdom'
import { RendererImpl } from 'renderer/common'
import { Lens } from 'core/lens'
import { assert } from '@raycenity/misc-ts'
import { Context } from 'core/hooks/intrinsic/context'

type PendingUpdateDetails = string

export interface VComponent<Props = any> {
  readonly type: 'component'
  readonly key: string

  props: Props
  construct: (props: Props) => VNode
  node: VNode | null
  readonly state: any[]
  readonly providedContexts: Map<Context, any>
  /** We can cache the ancestor's provided context because parents / ancestors don't change */
  readonly consumedContexts: Map<Context, any>
  readonly stateTrackers: Map<Lens<any>, (newValue: any, debugPath: string) => void>
  readonly effects: Array<() => void>
  readonly updateDestructors: Array<() => void>
  nextUpdateDestructors: Array<() => void>
  readonly permanentDestructors: Array<() => void>

  /** children in the `VComponent` build tree (and `VNode` tree) */
  readonly children: Map<string, VComponent>
  readonly renderer: RendererImpl<any, any>

  isBeingUpdated: boolean
  isFresh: boolean
  isDead: boolean
  hasPendingUpdates: boolean
  recursiveUpdateStackTrace: PendingUpdateDetails[]
  nextStateIndex: number
}

const RENDERER_STACK: Array<RendererImpl<any, any>> = []
let VCOMPONENT_STACK: VComponent[] = []

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

/** **Warning:** While vcomponents higher in the stack are guaranteed to be ancestors of vcomponents lower,
 * there may be gaps, since indirect children aren't part of the stack.
 */
export function * iterVComponentsStackTopDown (): Generator<VComponent> {
  for (let i = VCOMPONENT_STACK.length - 1; i >= 0; i--) {
    yield VCOMPONENT_STACK[i]
  }
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

export function VRoot<T extends VNodeNode> (renderer: RendererImpl<any, any>, construct: () => T): T {
  const stack = VCOMPONENT_STACK
  VCOMPONENT_STACK = []
  const node = withRenderer(renderer, construct)
  VNodeNode.update(node, 'init:')
  VCOMPONENT_STACK = stack
  return node
}

export function VComponent<Props> (key: string, props: Props, construct: (props: Props) => VNode): VComponent {
  if (VCOMPONENT_STACK.length !== 0) {
    const parent = getVComponent()
    // parent is being created = if there are any existing children, they're not being reused, they're a conflict
    if (!VComponent.isBeingCreated(parent)) {
      for (const [key, vcomponent] of parent.children) {
        // If the componennt was already reused this update, it's a conflict. We fallthrough to newVComponent which throws the error
        if (!vcomponent.isFresh) {
          vcomponent.props = props
          vcomponent.construct = construct
          vcomponent.isFresh = true
          VComponent.update(vcomponent, `child ${key}`)
          return vcomponent
        }
      }
    }
  }

  return VComponent.create(key, props, construct)
}

export module VComponent {
  export function create<Props> (key: string, props: Props, construct: (props: Props) => VNode): VComponent {
    // Create JS object
    const vcomponent: VComponent<Props> = {
      type: 'component',
      key,

      props,
      construct,
      node: null,
      state: [],
      providedContexts: new Map(),
      consumedContexts: new Map(),
      stateTrackers: new Map(),
      effects: [],
      updateDestructors: [],
      nextUpdateDestructors: [],
      permanentDestructors: [],

      children: new Map(),
      renderer: getRenderer(),

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
      if (currentRenderer.root !== null) {
        throw new Error('there can only be one root component')
      }
      currentRenderer.root = vcomponent
      vcomponent.isFresh = false
    } else {
      const parent = getVComponent()
      if (parent.children.has(key)) {
        throw new Error(`multiple components with the same parent and key: ${key}. Please assign different keys so that devolve-ui can distinguish the components in updates`)
      }
      parent.children.set(key, vcomponent)
    }

    return vcomponent
  }

  export function update (vcomponent: VComponent, details: PendingUpdateDetails | null): void {
    if (vcomponent.isBeingUpdated) {
      // Delay until after this update, especially if there are multiple triggered updates since we only have to update once more
      vcomponent.hasPendingUpdates = true
      if (isDebugMode() && details !== null) {
        vcomponent.recursiveUpdateStackTrace.push(details)
      }
    } else if (vcomponent.node === null) {
      // Do construct (component and renderer are already set)
      withRenderer(vcomponent.renderer, () => doUpdate(vcomponent, () => {
        // Actually do construct and set vcomponent.node
        const node = vcomponent.construct(vcomponent.props)
        vcomponent.node = node
        if (typeof node !== 'object' || Array.isArray(node)) {
          throw new Error('JSX components can only be nodes. Call this function normally, not with JSX')
        }

        // Create pixi if pixi component and on web
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
      }))
    } else {
      // Reset
      runUpdateDestructors(vcomponent)
      vcomponent.nextStateIndex = 0
      vcomponent.providedContexts.clear()

      // Do construct
      // We also need to use VComponent's renderer because the current renderer might be different
      withRenderer(vcomponent.renderer, () => doUpdate(vcomponent, () => {
        const node = vcomponent.construct(vcomponent.props)
        vcomponent.node = node

        // Update pixi if pixi component and on web
        if (node.type === 'pixi' && node.pixi !== 'terminal') {
          const pixiComponent: PixiComponent<any> = vcomponent.construct as PixiComponent<any>
          pixiComponent.lifecycle.update?.(node.pixi)
        }
      }))
      vcomponent.renderer.invalidate(vcomponent.node)
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
      if (vcomponent.recursiveUpdateStackTrace.length > GLOBAL_COMPONENT_OPTS.maxRecursiveUpdatesBeforeLoopDetected) {
        throw new Error(`update loop detected:\n${vcomponent.recursiveUpdateStackTrace.join('\n')}`)
      }
      update(vcomponent, null)
    } else if (isDebugMode()) {
      vcomponent.recursiveUpdateStackTrace = []
    }
  }

  function clearFreshAndRemoveStaleChildren (vcomponent: VComponent): void {
    // Need to copy map because we're going to remove some entries
    for (const [childKey, child] of new Map(vcomponent.children)) {
      if (child.isFresh) {
        child.isFresh = false
      } else {
        destroy(child)
        vcomponent.children.delete(childKey)
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

  /** Makes the given component update when the given state changes. hookId is used for the stack trace on update loop */
  export function trackState<T> (vcomponent: VComponent, state: Lens<T>, hookId: string): void {
    assert(!vcomponent.stateTrackers.has(state), `state ${hookId} is already tracked`)
    const stateTracker = (newValue: T, debugPath: string): void => {
      const stackTrace = isDebugMode()
        ? (new Error().stack?.replace('\n', '  \n') ?? 'could not get stack, new Error().stack is undefined')
        : 'omitted in production'
      update(vcomponent, `${hookId}-${debugPath}\n${stackTrace}`)
    }
    vcomponent.stateTrackers.set(state, stateTracker)
    Lens.onSet(state, stateTracker)
  }

  function setConsumedContexts (vcomponent: VComponent, context: Context, value: any): void {
    if (vcomponent.consumedContexts.has(context) && vcomponent.consumedContexts.get(context) !== value) {
      vcomponent.consumedContexts.set(context, value)
      update(vcomponent, `changed-provided-context-to-${context.debugId}`)
    }
    for (const child of vcomponent.children.values()) {
      setConsumedContexts(child, context, value)
    }
  }

  /** Sets the value of the provided context in the component, and consumed context in child components */
  export function setProvidedContext (vcomponent: VComponent, context: Context, value: any): void {
    assert(!vcomponent.providedContexts.has(context), 'setProvidedContext called multiple times with the same provided context in the same update')
    vcomponent.providedContexts.set(context, value)
    for (const child of Object.values(vcomponent.children)) {
      setConsumedContexts(child, context, value)
    }
  }

  export function isBeingCreated (vcomponent: VComponent): boolean {
    return vcomponent.node === null
  }
}

export interface GlobalComponentOpts {
  maxRecursiveUpdatesBeforeLoopDetected: number
  isDebugMode: boolean
}

export const DEFAULT_GLOBAL_COMPONENT_OPTS: GlobalComponentOpts = {
  maxRecursiveUpdatesBeforeLoopDetected: 100,
  isDebugMode: true
}

const GLOBAL_COMPONENT_OPTS: GlobalComponentOpts = { ...DEFAULT_GLOBAL_COMPONENT_OPTS }

export function setGlobalComponentOpts (opts: Partial<GlobalComponentOpts>): void {
  Object.assign(GLOBAL_COMPONENT_OPTS, opts)
}

export function isDebugMode (): boolean {
  return GLOBAL_COMPONENT_OPTS.isDebugMode
}
