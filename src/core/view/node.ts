import { VView } from 'core'
import { VComponent } from 'core/component'
import { assert } from '@raycenity/misc-ts'

export type VNode = VView | VComponent
export module VNode {
  let NEXT_ID: number = 0

  export function nextId (): number {
    return ++NEXT_ID
  }

  export function update (node: VNode, updatePath: string): void {
    updatePath += `/${node.key ?? ''}`
    if (node.type === 'component') {
      VComponent.update(node, updatePath)
    } else if (node.type === 'box') {
      node.children.forEach((child, index) => {
        const updateSubpath = `${updatePath}[${index}]`
        update(child, updateSubpath)
      })
    }
  }

  export function view (node: VNode): VView {
    if (node.type === 'component') {
      assert(node.node !== null, `tried to get view from uninitialized component: ${node.key}. It should've been initialized earlier`)
      return view(node.node)
    } else {
      return node
    }
  }
}
