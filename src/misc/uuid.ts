export type UUID = string

export module UUID {
  // crypto.randomUuid() is not available in insecure contexts,
  // but the app should still work in insecure contexts.
  // https://stackoverflow.com/a/2117523/2800218
  // LICENSE: https://creativecommons.org/licenses/by-sa/4.0/legalcode
  export function v4 (): UUID {
    // @ts-expect-error
    // eslint-disable-next-line @typescript-eslint/restrict-plus-operands
    return ([1e7] + -1e3 + -4e3 + -8e3 + -1e11).replace(/[018]/g, c =>
      (c ^ crypto.getRandomValues(new Uint8Array(1))[0] & 15 >> c / 4).toString(16)
    )
  }
}
