export interface SubLayout {
  direction?: 'horizontal' | 'vertical' | null
  gap?: number
  alignHorizontal?: 'start' | 'center' | 'end' | null
  alignVertical?: 'start' | 'center' | 'end' | null
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

export interface ParentBounds {
  boundingBox: BoundingBox,
  sublayout: SubLayout
}

export type Bounds = (parent: ParentBounds) => BoundingBox

export type Measurement =
  `${number}px` |
  `${number}em` |
  `${number}%`

export type BoundsSpec =
  { x?: Measurement, y?: Measurement, z?: number, anchorX?: number, anchorY?: number, width?: Measurement, height?: Measurement }


export function Bounds(spec: BoundsSpec): Bounds {
  if ('x' in spec && 'y' in spec && 'z' in spec && 'anchorX' in spec && 'anchorY' in spec && 'width' in spec && 'height' in spec) {
    return parent => ({
      x: reifyX(parent, spec.x),
      y: reifyY(parent, spec.y),
      z: spec.z ?? parent.boundingBox.z + Bounds.BOX_Z,
      anchorX: spec.anchorX ?? 0,
      anchorY: spec.anchorY ?? 0,
      width: reifyWidth(parent, spec.width),
      height: reifyHeight(parent, spec.height),
    })
  } else {
    throw new Error(`invalid bounds spec: ${spec}`)
  }
}

function reifyX(parent: ParentBounds, x: Measurement): number {
  if (x.endsWith('%')) {
    if (parent.boundingBox.width === undefined) {
      throw new Error(`cannot reify percent x-position ${x} because parent width is unknown`)
    }
    return (parent.boundingBox.width * parseFloat(x) / 100)
  } else if (x.endsWith('em')) {
    return parseFloat(x)
  } else if (x.endsWith('px')) {
    return parseFloat(x) / PIXEL_SIZE
  } else {
    throw new Error(`invalid measurement: ${x}`)
  }
}

export module ParentBounds {
  export function equals (a: ParentBounds, b: ParentBounds): boolean {
    return JSON.stringify(a) === JSON.stringify(b)
  }
}

export module Bounds {
  export const BOX_Z = 0.0001
  export const DELTA_Z = 0.0000001
}
