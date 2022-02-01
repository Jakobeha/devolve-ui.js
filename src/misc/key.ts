export interface Key {
  name: string
  ctrl: boolean
  meta: boolean
  shift: boolean
}

export module Key {
  export function fromKeyboardEvent (event: KeyboardEvent): Key {
    return {
      name: nameFromEventKey(event.key),
      ctrl: event.ctrlKey,
      meta: event.altKey,
      shift: event.shiftKey
    }
  }

  export function nameFromEventKey (eventKey: string): string {
    switch (eventKey) {
      case 'ArrowLeft':
        return 'left'
      case 'ArrowRight':
        return 'right'
      case 'ArrowUp':
        return 'up'
      case 'ArrowDown':
        return 'down'
      default:
        return eventKey.toLowerCase()
    }
  }
}
