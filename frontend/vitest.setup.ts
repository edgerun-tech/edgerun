import '@testing-library/dom'
import '@testing-library/react'
import { cleanup } from '@testing-library/react'
import { afterEach, beforeEach, vi } from 'vitest'

afterEach(() => {
  cleanup()
})

globalThis.ResizeObserver = vi.fn().mockImplementation(() => ({
  observe: vi.fn(),
  unobserve: vi.fn(),
  disconnect: vi.fn(),
}))

globalThis.matchMedia = vi.fn().mockImplementation((query) => ({
  matches: false,
  media: query,
  onchange: null,
  addListener: vi.fn(),
  removeListener: vi.fn(),
  addEventListener: vi.fn(),
  removeEventListener: vi.fn(),
  dispatchEvent: vi.fn(),
}))

const mockScrollTo = vi.fn()
globalThis.scrollTo = mockScrollTo

const originalConsoleError = console.error
beforeEach(() => {
  console.error = (...args: unknown[]) => {
    const firstArg = args[0]
    if (
      typeof firstArg === 'string' &&
      (firstArg.includes('Warning: An update to') ||
        firstArg.includes('Warning: React12') ||
        firstArg.includes('Warning: unstable_'))
    ) {
      return
    }
    originalConsoleError.call(console, ...args)
  }
})

afterEach(() => {
  console.error = originalConsoleError
})