// SPDX-License-Identifier: Apache-2.0
import linksConfig from '../config/site-links.json'

export type SiteLinks = {
  repository: string
  community: {
    github: string
    x: string
    discord: string
  }
}

export const siteLinks = linksConfig as SiteLinks
