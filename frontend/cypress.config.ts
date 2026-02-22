// SPDX-License-Identifier: Apache-2.0
import { defineConfig } from 'cypress'

export default defineConfig({
  env: {},
  allowCypressEnv: false,
  video: false,
  screenshotOnRunFailure: true,
  e2e: {
    baseUrl: 'http://127.0.0.1:4173',
    supportFile: false,
    specPattern: 'cypress/e2e/**/*.cy.{js,ts}'
  }
})
