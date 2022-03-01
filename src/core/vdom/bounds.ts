export type LayoutDirection = 'horizontal' | 'vertical' | 'overlap'

export type Measurement =
  number |
  `${number}` |
  `${number}px` |
  `${number}%` |
  `${number}% ${'+' | '-'} ${number}` |
  `${number}% ${'+' | '-'} ${number}px` |
  `${number}% ${'+' | '-'} ${number}px ${'+' | '-'} ${number}`

export type LayoutPosition1D =
  'global-absolute' |
  'local-absolute' |
  'relative'

export type LayoutPosition =
  LayoutPosition1D |
  { x: LayoutPosition1D, y: LayoutPosition1D }

export interface SubLayout {
  direction?: LayoutDirection
  gap?: Measurement
  custom?: any
}

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
  sublayout: SubLayout
  columnSize: Size
}

export type Bounds = (parent: ParentBounds, prevSibling: BoundingBox | null) => BoundingBox

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
    x: applyLayoutX(parent, prevSibling, spec.layout ?? 'relative', reifyX(parent, spec.x ?? 0)),
    y: applyLayoutY(parent, prevSibling, spec.layout ?? 'relative', reifyY(parent, spec.y ?? 0)),
    z: spec.z ?? parent.boundingBox.z + Bounds.BOX_Z,
    anchorX: spec.anchorX ?? 0,
    anchorY: spec.anchorY ?? 0,
    width: spec.width === undefined ? undefined : reifyX(parent, spec.width),
    height: spec.height === undefined ? undefined : reifyY(parent, spec.height)
  })
}

function reifyX (parent: ParentBounds, x: Measurement): number {
  if (typeof x === 'number') {
    return x
  } else if (/^[0-9]+$/.test(x)) {
    return parseInt(x)
  } else if (/^[0-9]*\.[0-9]+$/.test(x)) {
    return parseFloat(x)
  } else if (x.endsWith('%')) {
    if (parent.boundingBox.width === undefined) {
      throw new Error(`cannot convert percent ${x} to number because parent width is unknown`)
    }
    return (parent.boundingBox.width * parseFloat(x) / 100)
  } else if (x.endsWith('px')) {
    return parseFloat(x) / parent.columnSize.width
  } else if (x.includes('+')) {
    const [left, right] = x.split('+')
    return reifyX(parent, left.trimEnd() as Measurement) + reifyX(parent, right.trimStart() as Measurement)
  } else if (x.includes('-')) {
    const [left, right] = x.split('-')
    return reifyX(parent, left.trimEnd() as Measurement) - reifyX(parent, right.trimStart() as Measurement)
  } else {
    throw new Error(`invalid measurement: ${x}`)
  }
}

function reifyY (parent: ParentBounds, y: Measurement): number {
  if (typeof y === 'number') {
    return y
  } else if (/^[0-9]+$/.test(y)) {
    return parseInt(y)
  } else if (/^[0-9]*\.[0-9]+$/.test(y)) {
    return parseFloat(y)
  } else if (y.endsWith('%')) {
    if (parent.boundingBox.height === undefined) {
      throw new Error(`cannot convert percent ${y} to number because parent height is unknown`)
    }
    return (parent.boundingBox.height * parseFloat(y) / 100)
  } else if (y.endsWith('px')) {
    return parseFloat(y) / parent.columnSize.height
  } else if (y.includes('+')) {
    const [left, right] = y.split('+')
    return reifyY(parent, left.trimEnd() as Measurement) + reifyY(parent, right.trimStart() as Measurement)
  } else if (y.includes('-')) {
    const [left, right] = y.split('-')
    return reifyY(parent, left.trimEnd() as Measurement) - reifyY(parent, right.trimStart() as Measurement)
  } else {
    throw new Error(`invalid measurement: ${y}`)
  }
}

function applyLayoutX (parent: ParentBounds, prevSibling: BoundingBox | null, layout: LayoutPosition, reified: number): number {
  const layout1D = typeof layout === 'string' ? layout : layout.x
  switch (layout1D) {
    case 'global-absolute':
      return reified
    case 'local-absolute':
      return reified + parent.boundingBox.x
    case 'relative':
      switch (parent.sublayout.direction) {
        case 'horizontal':
          return reified + (prevSibling?.x ?? parent.boundingBox.x)
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

function applyLayoutY (parent: ParentBounds, prevSibling: BoundingBox | null, layout: LayoutPosition, reified: number): number {
  const layout1D = typeof layout === 'string' ? layout : layout.y
  switch (layout1D) {
    case 'global-absolute':
      return reified
    case 'local-absolute':
      return reified + parent.boundingBox.y
    case 'relative':
      switch (parent.sublayout.direction) {
        case 'horizontal':
          return reified + parent.boundingBox.y
        case 'vertical':
          return reified + (prevSibling?.y ?? parent.boundingBox.y)
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

export module ParentBounds {
  export function equals (a: ParentBounds, b: ParentBounds): boolean {
    return JSON.stringify(a) === JSON.stringify(b)
  }
}

export module BoundingBox {
  export function equals (a: BoundingBox | null, b: BoundingBox | null): boolean {
    return JSON.stringify(a) === JSON.stringify(b)
  }

  export function toRectangle (bounds: BoundingBox & Size): Rectangle {
    return {
      left: bounds.x - (bounds.anchorX * bounds.width),
      top: bounds.y - (bounds.anchorY * bounds.height),
      width: bounds.width,
      height: bounds.height
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
}
