export class BackRef<Parent extends object, Id> {
  private readonly _ref: WeakRef<Parent>
  readonly id: Id

  constructor (parent: Parent, id: Id) {
    this._ref = new WeakRef(parent)
    this.id = id
  }

  get (): Parent {
    const parent = this._ref.deref()
    if (parent === undefined) {
      throw new Error('back reference\'s parent has been collected but the reference is still being used')
    }
    return parent
  }
}
