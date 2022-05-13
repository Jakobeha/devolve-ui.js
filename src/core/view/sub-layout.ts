import { BoundingBox, LayoutDirection, Measurement, ParentBounds, Rectangle } from 'core/view/bounds'
import { assert } from '@raycenity/misc-ts'

export interface CustomSubLayout {
  [key: string | number | symbol]: any
}

export type CustomDelayedSubLayout =
  CustomSubLayout
  | ((myBoundingBox: BoundingBox, parent: ParentBounds, prevSibling: Rectangle | null) => CustomSubLayout)

export interface SubLayout {
  direction?: LayoutDirection
  gap?: Measurement
}

export interface DelayedSubLayout extends SubLayout {
  store?: string
  keep?: string[]
  custom?: CustomDelayedSubLayout
}

export interface ParentSubLayout extends SubLayout {
  stored?: { [name: string]: BoundingBox }
  custom?: CustomSubLayout
}

export module DelayedSubLayout {
  function calculateStored (store: string | undefined, keep: string[] | undefined, bounds: BoundingBox, parentBounds: ParentBounds): { [key: string]: BoundingBox } {
    const stored: { [name: string]: BoundingBox } = {}
    if (store !== undefined) {
      stored[store] = bounds
    }
    if (keep !== undefined) {
      for (const key of keep) {
        assert(parentBounds.sublayout.stored !== undefined, `bad layout: parent doesn't have any stored bounding boxes, including the one we wanted to keep: ${key}`)
        assert(key in parentBounds.sublayout.stored, `bad layout: parent doesn't have bounding box to keep: ${key}`)
        stored[key] = parentBounds.sublayout.stored[key]
      }
    }
    return stored
  }

  export function resolve (sublayout: DelayedSubLayout, bounds: BoundingBox, parentBounds: ParentBounds, siblingBounds: Rectangle | null): ParentSubLayout {
    const { store, keep, custom, ...attrs } = sublayout

    return {
      custom: typeof custom === 'function' ? custom(bounds, parentBounds, siblingBounds) : custom,
      stored: calculateStored(store, keep, bounds, parentBounds),
      ...attrs
    }
  }
}
