/**
 * Capability protocol helpers.
 * Uses generated protobuf types for capabilities and trust messages.
 */

import { edgerun as edgerunCap } from "@/gen/edgerun/v0/capability"
import { edgerun as edgerunTrust } from "@/gen/edgerun/v0/trust"
import { edgerun } from "@/gen/edgerun/v0/common"
import { protocolClient } from "./client"

export type CapabilityDescriptor = edgerunCap.v0.capability.CapabilityDescriptor
export type CapabilityGrant = edgerunCap.v0.capability.CapabilityGrant
export type CapabilitySelector = edgerunCap.v0.capability.CapabilitySelector
export type CapabilityConstraint = edgerunCap.v0.capability.CapabilityConstraint
export type DelegationRecord = edgerunTrust.v0.trust.DelegationRecord

export async function listAvailableCapabilities(
  nodeId?: string,
): Promise<CapabilityDescriptor[]> {
  const path = nodeId
    ? `/protocol/capabilities?node=${nodeId}`
    : "/protocol/capabilities"
  const response = await protocolClient.send({ method: "GET", path })
  if (response.status !== 200) return []
  const items = JSON.parse(new TextDecoder().decode(response.body)) as Array<any>
  return items.map((obj) => edgerunCap.v0.capability.CapabilityDescriptor.fromObject(obj))
}

export async function listGrantsForApp(
  appId: string,
): Promise<CapabilityGrant[]> {
  const response = await protocolClient.send({
    method: "GET",
    path: `/protocol/app/${appId}/grants`,
  })
  if (response.status !== 200) return []
  const items = JSON.parse(new TextDecoder().decode(response.body)) as Array<any>
  return items.map((obj) => edgerunCap.v0.capability.CapabilityGrant.fromObject(obj))
}
