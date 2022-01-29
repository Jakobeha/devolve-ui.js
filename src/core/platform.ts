export type Platform = 'cli' | 'web'

export const PLATFORM: Platform = typeof window === 'undefined' ? 'cli' : 'web'
