// SPDX-License-Identifier: Apache-2.0

function requiredEnv(name, value, errors) {
  if (!value || !value.trim()) {
    errors.push(`Missing required env var: ${name}`)
  }
}

function assertHttps(name, value, errors) {
  if (!value) return
  if (!/^https:\/\//.test(value)) {
    errors.push(`${name} must be an https URL: received "${value}"`)
  }
}

const version = process.env.EDGERUN_VERSION?.trim() || ''
const buildNumber = process.env.EDGERUN_BUILD_NUMBER?.trim() || ''
const siteUrl = process.env.EDGERUN_SITE_URL?.trim() || ''
const siteDomain = process.env.EDGERUN_SITE_DOMAIN?.trim() || ''

const errors = []

requiredEnv('EDGERUN_VERSION', version, errors)
requiredEnv('EDGERUN_BUILD_NUMBER', buildNumber, errors)
requiredEnv('EDGERUN_SITE_URL', siteUrl, errors)
requiredEnv('EDGERUN_SITE_DOMAIN', siteDomain, errors)

if (siteUrl) assertHttps('EDGERUN_SITE_URL', siteUrl, errors)
if (siteUrl && siteDomain) {
  try {
    const hostname = new URL(siteUrl).hostname
    if (hostname !== siteDomain) {
      errors.push(`EDGERUN_SITE_DOMAIN "${siteDomain}" must match EDGERUN_SITE_URL hostname "${hostname}"`)
    }
  } catch {
    errors.push(`EDGERUN_SITE_URL is not a valid URL: "${siteUrl}"`)
  }
}

if (errors.length) {
  console.error('production build validation failed:')
  for (const error of errors) {
    console.error(`- ${error}`)
  }
  process.exit(1)
}

console.log('production build validation passed')
