export function lazy<T> (factory: () => T): () => T {
  let value: T | undefined
  return () => {
    if (value === undefined) {
      value = factory()
    }
    return value
  }
}

export function lazyAsync<T> (factory: () => Promise<T>): () => Promise<T> {
  let value: T | undefined
  return async () => {
    if (value === undefined) {
      value = await factory()
    }
    return value
  }
}
