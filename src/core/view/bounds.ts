import { assert } from '@raycenity/misc-ts'
import { ParentSubLayout } from 'core/view/sub-layout'

export type LayoutDirection = 'horizontal' | 'vertical' | 'overlap'

export type Measurement =
  Measurement2 |
  `${Measurement2} ${'+' | '-'} ${Measurement3}` |
  `${Measurement2} ${'+' | '-'} ${Measurement3} ${'+' | '-'} ${Measurement4}` |
  `${Measurement2} ${'+' | '-'} ${Measurement3} ${'+' | '-'} ${Measurement4} ${'+' | '-'} ${Measurement5}`

type Measurement2 = 'prev' | Measurement3
type Measurement3 = `${number}%` | Measurement4
type Measurement4 = `${number}px` | Measurement5
type Measurement5 = `${number}` | number

export type LayoutPosition1D =
  'global-absolute' |
  'local-absolute' |
  'relative'

export type LayoutPosition =
  LayoutPosition1D |
  { x: LayoutPosition1D, y: LayoutPosition1D }

export interface BoundingBox {
  x: number
  y: number
  z: number
  anchorX: number
  anchorY: number
  width?: number
  height?: number
}

export interface Size {
  width: number
  height: number
}

export interface Rectangle {
  left: number
  top: number
  width: number
  height: number
}

export interface ParentBounds {
  boundingBox: BoundingBox
  sublayout: ParentSubLayout
  columnSize: Size
}

export type Bounds = (parent: ParentBounds, prevSibling: Rectangle | null) => BoundingBox

export interface FullBoundsSpec {
  layout?: LayoutPosition
  x?: Measurement
  y?: Measurement
  z?: number
  anchorX?: number
  anchorY?: number
  width?: Measurement
  height?: Measurement
}

export type BoundsSpec = FullBoundsSpec

export function Bounds (spec: BoundsSpec): Bounds {
  return (parent, prevSibling) => ({
    x: applyLayoutX(parent, prevSibling, spec.layout, reifyX(parent, 'not-applicable', spec.x)),
    y: applyLayoutY(parent, prevSibling, spec.layout, reifyY(parent, 'not-applicable', spec.y)),
    z: spec.z ?? parent.boundingBox.z + Bounds.BOX_Z,
    anchorX: spec.anchorX ?? 0,
    anchorY: spec.anchorY ?? 0,
    width: spec.width === undefined ? undefined : reifyX(parent, prevSibling?.width ?? null, spec.width),
    height: spec.height === undefined ? undefined : reifyY(parent, prevSibling?.height ?? null, spec.height)
  })
}

function reifyX (parent: ParentBounds, prevSibling: number |'not-applicable' | null, x: Measurement | undefined): number {
  if (x === undefined) {
    return 0
  } else if (typeof x === 'number') {
    return x
  } else if (/^\d+$/.test(x)) {
    return parseInt(x)
  } else if (/^\d*\.\d+$/.test(x)) {
    return parseFloat(x)
  } else if (x.endsWith('%')) {
    if (parent.boundingBox.width === undefined) {
      throw new Error(`cannot convert percent ${x} to number because parent width is unknown`)
    }
    return (parent.boundingBox.width * parseFloat(x) / 100)
  } else if (x.endsWith('px')) {
    return parseFloat(x) / parent.columnSize.width
  } else if (x === 'prev') {
    if (prevSibling === 'not-applicable') {
      throw new Error('can\'t use \'prev\' for position or gap')
    } else if (prevSibling === null) {
      throw new Error('can\'t use \'prev\' for first child')
    } else {
      return prevSibling
    }
  } else if (x.includes('+')) {
    const [left, right] = x.split('+')
    return reifyX(parent, prevSibling, left.trimEnd() as Measurement) + reifyX(parent, prevSibling, right.trimStart() as Measurement)
  } else if (x.includes('-')) {
    const [left, right] = x.split('-')
    return reifyX(parent, prevSibling, left.trimEnd() as Measurement) - reifyX(parent, prevSibling, right.trimStart() as Measurement)
  } else {
    throw new Error(`invalid measurement: ${x}`)
  }
}

function reifyY (parent: ParentBounds, prevSibling: number | 'not-applicable' | null, y: Measurement | undefined): number {
  if (y === undefined) {
    return 0
  } else if (typeof y === 'number') {
    return y
  } else if (/^\d+$/.test(y)) {
    return parseInt(y)
  } else if (/^\d*\.\d+$/.test(y)) {
    return parseFloat(y)
  } else if (y.endsWith('%')) {
    if (parent.boundingBox.height === undefined) {
      throw new Error(`bad layout: cannot convert percent ${y} to number because parent height is unknown`)
    }
    return (parent.boundingBox.height * parseFloat(y) / 100)
  } else if (y.endsWith('px')) {
    return parseFloat(y) / parent.columnSize.height
  } else if (y === 'prev') {
    if (prevSibling === 'not-applicable') {
      throw new Error('can\'t use \'prev\' for position or gap')
    } else if (prevSibling === null) {
      throw new Error('can\'t use \'prev\' for first child')
    } else {
      return prevSibling
    }
  } else if (y.includes('+')) {
    const [left, right] = y.split('+')
    return reifyY(parent, prevSibling, left.trimEnd() as Measurement) + reifyY(parent, prevSibling, right.trimStart() as Measurement)
  } else if (y.includes('-')) {
    const [left, right] = y.split('-')
    return reifyY(parent, prevSibling, left.trimEnd() as Measurement) - reifyY(parent, prevSibling, right.trimStart() as Measurement)
  } else {
    throw new Error(`invalid measurement: ${y}`)
  }
}

function applyLayoutX (parent: ParentBounds, prevSibling: Rectangle | null, layout: LayoutPosition | undefined, reified: number): number {
  const layout1D = typeof layout === 'string' || typeof layout === 'undefined' ? layout : layout.x
  switch (layout1D) {
    case 'global-absolute':
      return reified
    case 'local-absolute':
      return reified + parent.boundingBox.x
    case undefined:
    case 'relative':
      switch (parent.sublayout.direction) {
        case 'horizontal': {
          // Yes, we do want to reify the parent's sublayout with it's own bounds
          const gap = reifyX(parent, null, parent.sublayout.gap)
          return reified + (prevSibling !== null ? prevSibling.left + prevSibling.width + gap : getLayoutBoundingBoxLeft(parent.boundingBox))
        }
        case 'vertical':
          return reified + parent.boundingBox.x
        case 'overlap':
          return reified + parent.boundingBox.x
        case undefined:
          if (prevSibling !== null) {
            console.warn('no layout direction specified with multiple children, defaulting to overlap (applyLayoutX)')
          }
          return reified + parent.boundingBox.x
      }
  }
}

function applyLayoutY (parent: ParentBounds, prevSibling: Rectangle | null, layout: LayoutPosition | undefined, reified: number): number {
  const layout1D = typeof layout === 'string' || typeof layout === 'undefined' ? layout : layout.y
  switch (layout1D) {
    case 'global-absolute':
      return reified
    case 'local-absolute':
      return reified + parent.boundingBox.y
    case undefined:
    case 'relative':
      switch (parent.sublayout.direction) {
        case 'horizontal':
          return reified + parent.boundingBox.y
        case 'vertical': {
          // Yes, we do want to reify the parent's sublayout with it's own bounds
          const gap = reifyY(parent, null, parent.sublayout.gap)
          return reified + (prevSibling !== null ? prevSibling.top + prevSibling.height + gap : getLayoutBoundingBoxTop(parent.boundingBox))
        }
        case 'overlap':
          return reified + parent.boundingBox.y
        case undefined:
          if (prevSibling !== null) {
            console.warn('no layout direction specified with multiple children, defaulting to overlap (applyLayoutY)')
          }
          return reified + parent.boundingBox.y
      }
  }
}

export function getLayoutBoundingBoxLeft (bounds: BoundingBox): number {
  if (bounds.anchorX === 0) {
    return bounds.x
  } else if (bounds.width === undefined) {
    throw new Error('bad layout: bounds not anchored at left has no width, so we don\'t know where to put the child')
  } else {
    return bounds.x - (bounds.anchorX * bounds.width)
  }
}

export function getLayoutBoundingBoxTop (bounds: BoundingBox): number {
  if (bounds.anchorY === 0) {
    return bounds.y
  } else if (bounds.height === undefined) {
    throw new Error('bad layout: bounds not anchored at top has no height, so we don\'t know where to put the child')
  } else {
    return bounds.y - (bounds.anchorY * bounds.height)
  }
}

export module ParentBounds {
  export function equals (a: ParentBounds, b: ParentBounds): boolean {
    return JSON.stringify(a) === JSON.stringify(b)
  }
}

export module Rectangle {
  export function equals (a: Rectangle | null, b: Rectangle | null): boolean {
    return JSON.stringify(a) === JSON.stringify(b)
  }

  export function union (a: Rectangle | null, b: Rectangle | null): Rectangle | null {
    if (a === null) {
      return b
    } else if (b === null) {
      return a
    } else {
      const left = Math.min(a.left, b.left)
      const top = Math.min(a.top, b.top)
      const right = Math.max(a.left + a.width, b.left + b.width)
      const bottom = Math.max(a.top + a.height, b.top + b.height)
      return {
        left,
        top,
        width: right - left,
        height: bottom - top
      }
    }
  }

  export function intersection (a: Rectangle | null, b: Rectangle | null): Rectangle | null {
    if (a === null) {
      return null
    } else if (b === null) {
      return null
    } else {
      const left = Math.max(a.left, b.left)
      const top = Math.max(a.top, b.top)
      const right = Math.min(a.left + a.width, b.left + b.width)
      const bottom = Math.min(a.top + a.height, b.top + b.height)
      if (right < left || bottom < top) {
        return null
      } else {
        return {
          left,
          top,
          width: right - left,
          height: bottom - top
        }
      }
    }
  }
}

export module BoundingBox {
  export function toRectangle (bounds: BoundingBox & Size): Rectangle
  export function toRectangle (bounds: BoundingBox, size: Size): Rectangle
  export function toRectangle (bounds: BoundingBox, size?: Size): Rectangle {
    const width = bounds.width ?? size?.width
    const height = bounds.height ?? size?.height
    assert(width !== undefined && height !== undefined, 'toRectangle invalid arguments: bounds has no size and no size provided')
    return {
      left: bounds.x - (bounds.anchorX * width),
      top: bounds.y - (bounds.anchorY * height),
      width,
      height
    }
  }
}

export module Bounds {
  export const BOX_Z = 0.0001
  export const DELTA_Z = 0.0000001

  export const DEFAULT: Bounds = parent => ({
    x: parent.boundingBox.x,
    y: parent.boundingBox.y,
    z: parent.boundingBox.z + BOX_Z,
    anchorX: parent.boundingBox.anchorX,
    anchorY: parent.boundingBox.anchorY
  })

  export const FILL_X: Bounds = parent => ({
    x: parent.boundingBox.x,
    y: parent.boundingBox.y,
    z: parent.boundingBox.z + BOX_Z,
    anchorX: parent.boundingBox.anchorX,
    anchorY: parent.boundingBox.anchorY,
    width: parent.boundingBox.width
  })

  export const FILL_Y: Bounds = parent => ({
    x: parent.boundingBox.x,
    y: parent.boundingBox.y,
    z: parent.boundingBox.z + BOX_Z,
    anchorX: parent.boundingBox.anchorX,
    anchorY: parent.boundingBox.anchorY,
    height: parent.boundingBox.height
  })

  export const FILL: Bounds = parent => ({
    x: parent.boundingBox.x,
    y: parent.boundingBox.y,
    z: parent.boundingBox.z + BOX_Z,
    anchorX: parent.boundingBox.anchorX,
    anchorY: parent.boundingBox.anchorY,
    width: parent.boundingBox.width,
    height: parent.boundingBox.height
  })

  export const CENTER: Bounds = parent => {
    assert(parent.boundingBox.width !== undefined && parent.boundingBox.height !== undefined, 'bad layout: parent has no width or height, so we can\'t center it')

    return {
      x: parent.boundingBox.x + (parent.boundingBox.width / 2),
      y: parent.boundingBox.y + (parent.boundingBox.height / 2),
      z: parent.boundingBox.z + BOX_Z,
      anchorX: 0.5,
      anchorY: 0.5
    }
  }

  export const CENTER_X_FILL_Y: Bounds = parent => {
    assert(parent.boundingBox.width !== undefined, 'bad layout: parent has no width, so we can\'t center it')

    return {
      x: parent.boundingBox.x + (parent.boundingBox.width / 2),
      y: parent.boundingBox.y,
      z: parent.boundingBox.z + BOX_Z,
      anchorX: 0.5,
      anchorY: parent.boundingBox.anchorY,
      height: parent.boundingBox.height
    }
  }

  export const CENTER_Y_FILL_X: Bounds = parent => {
    assert(parent.boundingBox.height !== undefined, 'bad layout: parent has no height, so we can\'t center it')

    return {
      x: parent.boundingBox.x,
      y: parent.boundingBox.y + (parent.boundingBox.height / 2),
      z: parent.boundingBox.z + BOX_Z,
      anchorX: parent.boundingBox.anchorX,
      anchorY: 0.5,
      width: parent.boundingBox.width
    }
  }

  export const PREV: Bounds = (parent, prev) => {
    assert(prev !== null, 'bad layout: parent has no previous sibling')

    return {
      x: prev.left,
      y: prev.top,
      z: parent.boundingBox.z + BOX_Z,
      anchorX: 0,
      anchorY: 0,
      width: prev.width,
      height: prev.height
    }
  }

  export function addZ (z: number, bounds: Bounds): Bounds {
    return (parent, prev) => {
      const boundingBox = bounds(parent, prev)
      return {
        ...boundingBox,
        z: boundingBox.z + z
      }
    }
  }

  export function withBoundingBox (boundingBox: BoundingBox | string, bounds: Bounds): Bounds {
    return (parent, prev) => {
      let boundingBox_: BoundingBox
      if (typeof boundingBox === 'string') {
        assert(parent.sublayout.stored !== undefined, `bad layout: parent has no stored bounding boxes, tried to get one named ${boundingBox}`)
        assert(boundingBox in parent.sublayout.stored, `bad layout: parent has no stored bounding box named ${boundingBox}`)
        boundingBox_ = parent.sublayout.stored[boundingBox]
      } else {
        boundingBox_ = boundingBox
      }
      return bounds({ ...parent, boundingBox: boundingBox_ }, prev)
    }
  }
}
