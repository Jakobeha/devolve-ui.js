import { JSXPixiAttrs } from 'core/vdom/attrs'
import { VPixi } from 'core/vdom/node'
import type { DisplayObject } from 'pixi.js'

/** Manages a pixi {@link DisplayObject} in the virtual DOM. */
export interface PixiLifecycle<Pixi extends DisplayObject> {
  /** Called when the component is created (not updated), to create the {@link DisplayObject}. */
  mkPixi: (pixi: typeof import('pixi.js')) => Pixi
  /** Called when the component is updated (not created), to update the {@link DisplayObject}. */
  update?: (pixi: Pixi) => void
  /** Called when the component is removed, to perform any necessary destruction (e.g. free data) */
  destroy?: (pixi: Pixi) => void
}

/**
 * A component function which returns nodes containing pixi {@link DisplayObject}.
 * It keeps track of these nodes via `pixis` and `pixi` properties,
 * so they can be referenced by other `PixiComponent`s.
 * Call `PixiComponent` to create these.
 */
export interface PixiComponent<Pixi extends DisplayObject> {
  /** Use as a component */
  (): VPixi<Pixi>
  /** Returns all `Pixi`s from every component created from `C`, in the order they were created. */
  pixis: Pixi[]
  /** Helper to get the only `Pixi` created from `C` */
  // This is a getter so it has to be shorthand
  // eslint-disable-next-line @typescript-eslint/method-signature-style
  get pixi (): Pixi
  lifecycle: PixiLifecycle<Pixi>
}

/** Returns a component which generates nodes containing pixi {@link DisplayObject}s.
 * The `lifecycle` parameters determines how these nodes are created and updated.
 * `attrs` contains `getSize` to generate a size for use in laying out other nodes,
 * as well as other standard component attributes.
 */
export function PixiComponent<Pixi extends DisplayObject> (
  lifecycle: PixiLifecycle<Pixi>,
  attrs?: JSXPixiAttrs<Pixi>
): PixiComponent<Pixi> {
  const pixiComponent: PixiComponent<Pixi> = Object.assign(() => VPixi(attrs ?? {}), {
    pixis: [],
    get pixi (): Pixi {
      if (pixiComponent.pixis.length === 0) {
        throw new Error('no pixi DisplayObject from this component')
      } else if (pixiComponent.pixis.length > 1) {
        throw new Error('multiple pixi DisplayObjects from this component')
      } else {
        return pixiComponent.pixis[0]
      }
    },
    lifecycle
  })
  return pixiComponent
}
