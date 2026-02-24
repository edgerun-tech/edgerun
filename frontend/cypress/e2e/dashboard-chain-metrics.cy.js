// SPDX-License-Identifier: Apache-2.0
describe('dashboard chain metrics', () => {
  it('renders deterministic Solana chain values from test-only RPC mock', () => {
    cy.visit('/dashboard/index.html', {
      onBeforeLoad(win) {
        win.localStorage.setItem('edgerun.rpc.cluster', 'devnet')
        win.localStorage.setItem('edgerun.rpc.url', 'https://api.devnet.solana.com')
        win.__EDGERUN_RPC_CONFIG = {
          cluster: 'devnet',
          rpcUrl: 'https://api.devnet.solana.com',
          treasuryAccount: '',
          deployments: {}
        }
        win.__EDGERUN_SOLANA_RPC_HTTP_MOCK_ENABLED__ = true
        win.__EDGERUN_SOLANA_RPC_HTTP_MOCK__ = ({ method }) => {
          if (method === 'getSlot') return 320000001
          if (method === 'getBlockHeight') return 289999991
          if (method === 'getEpochInfo') return { epoch: 777 }
          if (method === 'getRecentPerformanceSamples') {
            return [{ numTransactions: 12_345, samplePeriodSecs: 10 }]
          }
          if (method === 'getSupply') {
            return { value: { total: 1_500_000_000_000_000 } }
          }
          throw new Error(`unexpected_mock_method_${method}`)
        }
      }
    })

    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)
    cy.contains('h1', 'Dashboard').should('be.visible')

    cy.get('[data-chain-field="slot"]').should('contain.text', '320,000,001')
    cy.get('[data-chain-field="blockHeight"]').should('contain.text', '289,999,991')
    cy.get('[data-chain-field="epoch"]').should('contain.text', '777')
    cy.get('[data-chain-field="tps"]').should('contain.text', '1234.50')
    cy.get('[data-chain-field="cluster"]').should('contain.text', 'devnet')
  })
})
