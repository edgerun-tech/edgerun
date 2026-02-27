// SPDX-License-Identifier: Apache-2.0
import { siteLinksConfig } from '../config/site-links'

export type SiteLinks = {
  repository: string
  community: {
    github: string
    x: string
    discord: string
  }
}

export const siteLinks = siteLinksConfig as SiteLinks
