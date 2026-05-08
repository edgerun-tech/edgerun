import { describe, it, expect } from 'vitest'
import { Capability, checkCapabilities, CAPABILITY_REGISTRY } from './capabilities'

describe('capabilities', () => {
  describe('CAPABILITY_REGISTRY', () => {
    it('should have all capabilities defined', () => {
      expect(CAPABILITY_REGISTRY).toHaveProperty(Capability.Identity)
      expect(CAPABILITY_REGISTRY).toHaveProperty(Capability.NodeConnection)
      expect(CAPABILITY_REGISTRY).toHaveProperty(Capability.NetworkAccess)
      expect(CAPABILITY_REGISTRY).toHaveProperty(Capability.Filesystem)
      expect(CAPABILITY_REGISTRY).toHaveProperty(Capability.HardwareSigning)
      expect(CAPABILITY_REGISTRY).toHaveProperty(Capability.Payments)
      expect(CAPABILITY_REGISTRY).toHaveProperty(Capability.VoiceCall)
    })

    it('should have correct metadata for each capability', () => {
      Object.values(CAPABILITY_REGISTRY).forEach((info) => {
        expect(info.id).toBeDefined()
        expect(info.label).toBeDefined()
        expect(info.description).toBeDefined()
        expect(info.icon).toBeDefined()
        expect(typeof info.requiresAuth).toBe('boolean')
      })
    })
  })

  describe('checkCapabilities', () => {
    const guestContext = { isGuest: false, hasIdentity: false, hasNode: false }

    it('should return all results for required and optional caps', () => {
      const result = checkCapabilities(
        [Capability.Filesystem],
        [Capability.NetworkAccess],
        guestContext
      )
      expect(result.all).toHaveLength(2)
      expect(result.granted).toContain(Capability.Filesystem)
      expect(result.granted).toContain(Capability.NetworkAccess)
    })

    it('should block identity caps when no identity', () => {
      const result = checkCapabilities([Capability.Identity], [], guestContext)
      expect(result.blocked).toHaveLength(1)
      expect(result.granted).toHaveLength(0)
    })

    it('should grant identity caps when has identity', () => {
      const ctx = { ...guestContext, hasIdentity: true }
      const result = checkCapabilities([Capability.Identity], [], ctx)
      expect(result.granted).toHaveLength(1)
    })

    it('should block payments when no identity or node', () => {
      const result = checkCapabilities([Capability.Payments], [], guestContext)
      expect(result.blocked).toHaveLength(1)
    })

    it('should grant payments when has identity and node', () => {
      const ctx = { ...guestContext, hasIdentity: true, hasNode: true }
      const result = checkCapabilities([Capability.Payments], [], ctx)
      expect(result.granted).toHaveLength(1)
    })

    it('should grant network access without auth', () => {
      const result = checkCapabilities([Capability.NetworkAccess], [], guestContext)
      expect(result.granted).toHaveLength(1)
    })

    it('should deduplicate duplicate capabilities', () => {
      const result = checkCapabilities(
        [Capability.NetworkAccess],
        [Capability.NetworkAccess],
        guestContext
      )
      expect(result.all).toHaveLength(1)
    })
  })
})