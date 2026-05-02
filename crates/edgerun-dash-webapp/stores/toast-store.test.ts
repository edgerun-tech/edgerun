import { describe, it, expect, beforeEach, vi } from 'vitest'
import { toastsStore, addToast, dismissToast, removeToast, hasOpenToastStore } from './toast-store'

describe('toast-store', () => {
  beforeEach(() => {
    toastsStore.set([])
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  describe('addToast', () => {
    it('should add a toast to the store', () => {
      addToast({ title: 'Test toast' })
      const toasts = toastsStore.get()
      expect(toasts).toHaveLength(1)
      expect(toasts[0].title).toBe('Test toast')
    })

    it('should generate a unique id', () => {
      const toast1 = addToast({ title: 'Toast 1' })
      const toast2 = addToast({ title: 'Toast 2' })
      expect(toast1.id).not.toBe(toast2.id)
    })

    it('should limit toasts to TOAST_LIMIT', () => {
      for (let i = 0; i < 5; i++) {
        addToast({ title: `Toast ${i}` })
      }
      expect(toastsStore.get()).toHaveLength(1)
    })

    it('should return dismiss and update functions', () => {
      const result = addToast({ title: 'Test' })
      expect(typeof result.dismiss).toBe('function')
      expect(typeof result.update).toBe('function')
    })
  })

  describe('dismissToast', () => {
    it('should dismiss a specific toast', () => {
      const { id } = addToast({ title: 'Test' })
      dismissToast(id)
      expect(toastsStore.get()[0].open).toBe(false)
    })

    it('should dismiss all toasts when no id provided', () => {
      addToast({ title: 'Test 1' })
      addToast({ title: 'Test 2' })
      dismissToast()
      toastsStore.get().forEach((t) => {
        expect(t.open).toBe(false)
      })
    })
  })

  describe('removeToast', () => {
    it('should remove a specific toast', () => {
      const { id } = addToast({ title: 'Test' })
      removeToast(id)
      expect(toastsStore.get()).toHaveLength(0)
    })

    it('should remove all toasts when no id provided', () => {
      addToast({ title: 'Test 1' })
      addToast({ title: 'Test 2' })
      removeToast()
      expect(toastsStore.get()).toHaveLength(0)
    })
  })

  describe('hasOpenToastStore', () => {
    it('should compute whether there are open toasts', () => {
      expect(hasOpenToastStore.get()).toBe(false)
      addToast({ title: 'Test', open: true })
      expect(hasOpenToastStore.get()).toBe(true)
    })
  })
})