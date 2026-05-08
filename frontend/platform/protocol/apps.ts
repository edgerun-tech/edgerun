/**
 * App protocol helpers.
 * Uses generated protobuf types for AppPackage, AppPrincipal, and app-related messages.
 */

import { edgerun as edgerunStream } from "@/gen/edgerun/v0/stream"
import { edgerun } from "@/gen/edgerun/v0/common"
import { protocolClient } from "./client"

export type AppPackage = edgerunStream.v0.stream.AppPackage
export type AppPrincipal = edgerunStream.v0.stream.AppPrincipal
export type AppIntent = edgerunStream.v0.stream.AppIntent

export async function fetchAppPackage(
  appId: string,
): Promise<AppPackage | null> {
  const response = await protocolClient.send({
    method: "GET",
    path: `/protocol/app/${appId}/package`,
  })
  if (response.status !== 200) return null
  const obj = JSON.parse(new TextDecoder().decode(response.body))
  return edgerunStream.v0.stream.AppPackage.fromObject(obj)
}

export async function listInstalledApps(): Promise<AppPackage[]> {
  const response = await protocolClient.send({
    method: "GET",
    path: "/protocol/apps",
  })
  if (response.status !== 200) return []
  const items = JSON.parse(new TextDecoder().decode(response.body)) as Array<any>
  return items.map((obj) => edgerunStream.v0.stream.AppPackage.fromObject(obj))
}
