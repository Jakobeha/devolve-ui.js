import { PLATFORM } from 'core/platform'
import { PixiComponent, VView, VNode } from 'core/view'
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

/** **Warning:** While components higher in the stack are guaranteed to be ancestors of components lower,
 * there may be gaps, since indirect children aren't part of the stack.
 */
export function * iterVComponentsStackTopDown (): Generator<VComponent> {
  for (let i = VCOMPONENT_STACK.length - 1; i >= 0; i--) {
    yield VCOMPONENT_STACK[i]
  }
}

function withVComponent<T> (component: VComponent, body: () => T): T {
  VCOMPONENT_STACK.push(component)
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
  const stack = VCOMPONENT_STACK
  VCOMPONENT_STACK = []
  const node = withRenderer(renderer, construct)
  VNode.update(node, 'init:')
  VCOMPONENT_STACK = stack
  return node
}

export function VComponent<Props> (key: string, props: Props, construct: (props: Props) => VNode): VComponent {
  if (VCOMPONENT_STACK.length !== 0) {
    const parent = getVComponent()
    // parent is being created = if there are any existing children, they're not being reused, they're a conflict
    if (!VComponent.isBeingCreated(parent)) {
      for (const [key, component] of parent.children) {
        // If the componennt was already reused this update, it's a conflict. We fallthrough to VComponent.create which throws the error
        if (!component.isFresh) {
          component.props = props
          component.construct = construct
          component.isFresh = true
          VComponent.update(component, `child ${key}`)
          return component
        }
      }
    }
  }

  return VComponent.create(key, props, construct)
}

export module VComponent {
  export function create<Props> (key: string, props: Props, construct: (props: Props) => VNode): VComponent {
    // Create JS object
    const component: VComponent<Props> = {
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
      currentRenderer.root = component
      component.isFresh = false
    } else {
      const parent = getVComponent()
      if (parent.children.has(key)) {
        throw new Error(`multiple components with the same parent and key: ${key}. Please assign different keys so that devolve-ui can distinguish the components in updates`)
      }
      parent.children.set(key, component)
    }

    return component
  }

  export function update (component: VComponent, details: PendingUpdateDetails | null): void {
    if (component.isBeingUpdated) {
      // Delay until after this update, especially if there are multiple triggered updates since we only have to update once more
      component.hasPendingUpdates = true
      if (isDebugMode() && details !== null) {
        component.recursiveUpdateStackTrace.push(details)
      }
    } else if (component.node === null) {
      // Do construct
      withRenderer(component.renderer, () => doUpdate(component, () => {
        // Actually do construct and set component.node
        const node = component.construct(component.props)
        component.node = node
        if (typeof node !== 'object' || Array.isArray(node) || !('type' in node)) {
          throw new Error('JSX components can only return nodes (views or other components). Call this function normally, not with JSX')
        }

        // Create pixi if pixi component and on web
        if (node.type === 'pixi') {
          if (PLATFORM === 'web') {
            const pixiComponent: PixiComponent<any> = component.construct as PixiComponent<any>
            const PIXI = (globalThis as unknown as { PIXI: typeof import('pixi.js') }).PIXI
            node.pixi = pixiComponent.lifecycle.mkPixi(PIXI)
            pixiComponent.pixis.push(node.pixi)
          } else {
            node.pixi = 'terminal'
          }
        }

        // Update children (if box or another component)
        VNode.update(node, details ?? '')
      }))
    } else {
      // Reset
      runUpdateDestructors(component)
      component.nextStateIndex = 0
      component.providedContexts.clear()

      // Do construct
      // We also need to use VComponent's renderer because the current renderer might be different
      withRenderer(component.renderer, () => doUpdate(component, () => {
        const node = component.construct(component.props)
        component.node = node

        // Update pixi if pixi component and on web
        if (node.type === 'pixi' && node.pixi !== 'terminal') {
          const pixiComponent: PixiComponent<any> = component.construct as PixiComponent<any>
          pixiComponent.lifecycle.update?.(node.pixi)
        }

        // Update children (if box or another component)
        VNode.update(node, details ?? '')
      }))
      component.renderer.invalidate(view(component))
    }
  }

  export function destroy (component: VComponent): void {
    assert(!component.isDead, 'sanity check: tried to destroy already dead component')
    assert(component.node !== null, 'sanity check: tried to destroy uninitialized component')

    component.renderer.invalidate(view(component))

    const node = component.node
    if (node.type === 'pixi' && node.pixi !== 'terminal') {
      const pixiComponent: PixiComponent<any> = component.construct as PixiComponent<any>
      pixiComponent.lifecycle.destroy?.(node.pixi)
      pixiComponent.pixis.splice(pixiComponent.pixis.indexOf(node.pixi), 1)
    }
    runPermanentDestructors(component)

    component.isDead = true
    component.node = null

    for (const child of Object.values(component.children)) {
      destroy(child)
    }
  }

  function doUpdate (component: VComponent, body: () => void): void {
    if (component.isDead) {
      throw new Error('sanity check: tried to update dead component')
    }

    withVComponent(component, () => {
      component.isBeingUpdated = true

      // This will update state, add events, etc.
      body()

      clearFreshAndRemoveStaleChildren(component)
      component.isBeingUpdated = false
      runEffects(component)
    })
    if (component.hasPendingUpdates) {
      component.hasPendingUpdates = false
      if (component.recursiveUpdateStackTrace.length > GLOBAL_COMPONENT_OPTS.maxRecursiveUpdatesBeforeLoopDetected) {
        throw new Error(`update loop detected:\n${component.recursiveUpdateStackTrace.join('\n')}`)
      }
      update(component, null)
    } else if (isDebugMode()) {
      component.recursiveUpdateStackTrace = []
    }
  }

  function clearFreshAndRemoveStaleChildren (component: VComponent): void {
    // Need to copy map because we're going to remove some entries
    for (const [childKey, child] of new Map(component.children)) {
      if (child.isFresh) {
        child.isFresh = false
      } else {
        destroy(child)
        component.children.delete(childKey)
      }
    }
  }

  function runEffects (component: VComponent): void {
    // Effects might add new effects
    // If there are pending updates, we don't want to run any effects, because they will be run in the pending update
    // Of course, effects can cause more pending updates
    while (component.effects.length > 0 && !component.hasPendingUpdates) {
      const effect = component.effects.pop()!
      effect()
    }
    // Child effects are taken care of
  }

  function runUpdateDestructors (component: VComponent): void {
    // Destructors might add new destructors
    while (component.updateDestructors.length > 0) {
      const destructor = component.updateDestructors.pop()!
      destructor()
    }
    component.updateDestructors.push(...component.nextUpdateDestructors)
    component.nextUpdateDestructors = []
    // Child update (and permanent if necessary) destructors are taken care of
  }

  function runPermanentDestructors (component: VComponent): void {
    // Destructors might add new destructors
    while (component.permanentDestructors.length > 0) {
      const destructor = component.permanentDestructors.pop()!
      destructor()
    }
    // Child permanent destructors are taken care of
  }

  /** Makes the given component update when the given state changes. hookId is used for the stack trace on update loop */
  export function trackState<T> (component: VComponent, state: Lens<T>, hookId: string): void {
    assert(!component.stateTrackers.has(state), `state ${hookId} is already tracked`)
    const stateTracker = (newValue: T, debugPath: string): void => {
      const stackTrace = isDebugMode()
        ? (new Error().stack?.replace('\n', '  \n') ?? 'could not get stack, new Error().stack is undefined')
        : 'omitted in production'
      update(component, `${hookId}-${debugPath}\n${stackTrace}`)
    }
    component.stateTrackers.set(state, stateTracker)
    Lens.onSet(state, stateTracker)
  }

  function setConsumedContexts (component: VComponent, context: Context, value: any): void {
    if (component.consumedContexts.has(context) && component.consumedContexts.get(context) !== value) {
      component.consumedContexts.set(context, value)
      update(component, `changed-provided-context-to-${context.debugId}`)
    }
    for (const child of component.children.values()) {
      setConsumedContexts(child, context, value)
    }
  }

  /** Sets the value of the provided context in the component, and consumed context in child components */
  export function setProvidedContext (component: VComponent, context: Context, value: any): void {
    assert(!component.providedContexts.has(context), 'setProvidedContext called multiple times with the same provided context in the same update')
    component.providedContexts.set(context, value)
    for (const child of Object.values(component.children)) {
      setConsumedContexts(child, context, value)
    }
  }

  function view (component: VComponent): VView {
    return VNode.view(component)
  }

  export function isBeingCreated (component: VComponent): boolean {
    return component.node === null
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
