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
