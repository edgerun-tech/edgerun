// SPDX-License-Identifier: Apache-2.0
function openReviewStep() {
  cy.get('button[role="tab"]').contains('3. Review + Run').click({ force: true })
  cy.get('button[role="tab"]').contains('3. Review + Run').should('have.attr', 'aria-selected', 'true')
  cy.get('[data-testid="run-step-review"]').should('be.visible')
}

describe('run job orchestration UX', () => {
  it('supports preset and custom module flows with clear I/O contract', () => {
    cy.visit('/run/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)

    cy.get('[data-testid="journey-overview"]').within(() => {
      cy.contains('Why This Helps You').should('be.visible')
      cy.contains('Guarantees You Get').should('be.visible')
      cy.contains('How To Use It').should('be.visible')
    })

    cy.contains('button', 'Use Recommended Demo').click({ force: true })
    cy.get('button[role="tab"]').contains('2. Define Inputs').should('have.attr', 'aria-selected', 'true')

    cy.get('button[role="tab"]').contains('1. Choose Module').click({ force: true })
    cy.get('[data-testid="run-step-choose"]').should('be.visible')

    cy.get('[aria-label="Preset App"]').should('be.visible')
    cy.get('[data-testid="preset-mode-panel"]').should('be.visible')
    cy.contains('Solana Vanity Address Generator').should('be.visible')

    cy.get('[aria-label="Submission Mode"]').select('Upload Custom Module')
    cy.get('[data-testid="custom-mode-panel"]').should('be.visible')
    cy.get('[aria-label="Custom Module Name"]').should('be.visible')

    cy.get('[aria-label="Submission Mode"]').select('Preset App')
    cy.get('[data-testid="preset-mode-panel"]').should('be.visible')

    cy.contains('button', 'Continue to Configure App').click({ force: true })
    cy.get('[data-testid="run-step-inputs"]').should('be.visible')
    cy.contains('h4', 'Platform Job Envelope').should('be.visible')
    cy.get('[data-testid="economic-guardrails"]').should('contain.text', 'Economic Guardrails Applied')
    cy.get('[data-testid="economic-guardrails"]').should('contain.text', 'lamports')
    cy.contains('h4', 'App Configuration: Solana Vanity Address Generator').should('be.visible')
    cy.get('[data-testid="vanity-app-fields"]').should('be.visible')

    cy.get('button[role="tab"]').contains('1. Choose Module').click({ force: true })
    cy.get('[aria-label="Preset App"]').select('JSON Transform Module')
    cy.contains('button', 'Continue to Configure App').click({ force: true })
    cy.get('[data-testid="run-step-inputs"]').within(() => {
      cy.contains('h4', 'App Configuration: JSON Transform Module').should('be.visible')
    })
    cy.get('[data-testid="vanity-app-fields"]').should('not.be.visible')

    cy.get('[aria-label="App Input Source"]').select('Raw JSON payload')
    cy.get('[data-testid="json-input-panel"]').should('be.visible')
    cy.get('[aria-label="Input JSON"]').should('contain.value', '"document"')

    cy.get('[aria-label="App Input Source"]').select('Upload input file')
    cy.get('[data-testid="file-input-panel"]').should('be.visible')

    cy.get('button[role="tab"]').contains('1. Choose Module').click({ force: true })
    cy.get('[aria-label="Preset App"]').select('Solana Vanity Address Generator')
    cy.contains('button', 'Continue to Configure App').click({ force: true })
    cy.get('[aria-label="App Input Source"]').select('Predefined fields')
    cy.get('[data-testid="predefined-input-panel"]').should('be.visible')
    cy.get('[data-testid="vanity-app-fields"]').should('be.visible')

    cy.get('[data-testid="input-clarity-panel"]').within(() => {
      cy.contains('Input:').should('be.visible')
      cy.contains('Output:').should('be.visible')
      cy.contains('Expected Behavior:').should('be.visible')
    })

    openReviewStep()
    cy.get('[data-testid="run-step-review"]').within(() => {
      cy.contains('h3', 'Input').should('be.visible')
      cy.contains('h3', 'Output').should('be.visible')
      cy.contains('h3', 'What Will Happen').should('be.visible')
    })
    cy.get('[data-testid="estimate-min-escrow"]').should('contain.text', 'lamports')
    cy.get('[data-testid="estimate-default-escrow"]').should('contain.text', 'SOL')
    cy.get('[data-testid="estimate-committee-quorum"]').invoke('text').should('match', /\d+\s*\/\s*\d+/)
    cy.get('[data-testid="estimate-required-lock"]').should('contain.text', 'lamports')
    cy.get('[data-testid="estimate-protocol-fee"]').should('contain.text', 'lamports')
    cy.get('[data-testid="estimate-payout-each"]').should('contain.text', 'lamports')
    cy.contains('Est. Fee').should('not.exist')
  })

  it('shows validation errors when required safety acknowledgement is missing', () => {
    cy.visit('/run/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)

    cy.get('button[role="tab"]').contains('2. Define Inputs').click({ force: true })
    cy.get('[data-testid="run-step-inputs"]').should('be.visible')

    cy.get('[aria-label="Allow worker seed exposure"]').uncheck({ force: true })

    openReviewStep()
    cy.contains('button', 'Submit Job').click({ force: true })

    cy.get('[data-testid="submit-error"]').should('be.visible')
    cy.get('[data-testid="validation-errors"]').should('be.visible')
    cy.get('[data-testid="validation-errors"]').contains('Distributed mode requires explicit worker seed exposure acknowledgement.').should('be.visible')
  })

  it('shows scheduler submission error for unreachable scheduler URL', () => {
    cy.visit('/run/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)

    cy.get('button[role="tab"]').contains('2. Define Inputs').click({ force: true })
    cy.get('[data-testid="run-step-inputs"]').should('be.visible')

    cy.get('[aria-label="Scheduler URL"]').clear().type('http://127.0.0.1:9999')
    cy.get('[aria-label="Allow worker seed exposure"]').check({ force: true })

    openReviewStep()
    cy.contains('button', 'Submit Job').click({ force: true })

    cy.get('[data-testid="submit-error"]').should('be.visible')
    cy.get('[data-testid="submit-error"]').contains('Submission failed: control ws connection failed').should('be.visible')
  })

  it('shows happy path success receipt when submission contract is valid', () => {
    cy.visit('/run/', {
      onBeforeLoad(win) {
        win.__EDGERUN_CONTROL_WS_MOCK_ENABLED__ = true
        win.__EDGERUN_CONTROL_WS_MOCK__ = ({ op, payload }) => {
          if (op === 'job.create') {
            expect(payload).to.have.property('runtime_id')
            expect(payload).to.have.property('escrow_lamports')
            expect(payload.escrow_lamports).to.be.at.least(1_000_000)
            return { job_id: 'job-cypress-live-001' }
          }
          if (op === 'job.status') {
            expect(payload).to.deep.equal({ job_id: 'job-cypress-live-001' })
            return {
              job_id: 'job-cypress-live-001',
              reports: [{ worker_pubkey: 'worker-a', output_hash: 'abc123', output_len: 16 }],
              failures: [],
              quorum: {
                quorum_reached: true,
                onchain_status: 'finalized',
                winning_output_hash: 'abc123'
              }
            }
          }
          throw new Error(`unexpected_op_${op}`)
        }
      }
    })
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)

    cy.get('button[role="tab"]').contains('2. Define Inputs').click({ force: true })
    cy.get('[data-testid="run-step-inputs"]').should('be.visible')
    cy.get('[aria-label="Scheduler URL"]').clear().type('https://api.edgerun.tech')
    cy.get('[aria-label="Allow worker seed exposure"]').check({ force: true })

    openReviewStep()
    cy.contains('button', 'Submit Job').click({ force: true })

    cy.get('[data-testid="submit-success"]:visible').should('exist')
    cy.get('[data-testid="submit-success"]:visible').contains('Submission Accepted').should('be.visible')
    cy.get('[data-testid="submit-success"]:visible').contains('job-cypress-live-001').should('be.visible')
    cy.get('[data-testid="submit-success"]:visible').contains('Receipt:').should('be.visible')
    cy.get('[data-testid="job-tracker-card"]').should('be.visible')
    cy.get('[data-testid="tracker-report-count"]').should('contain.text', '1')
    cy.get('[data-testid="tracker-quorum"]').should('contain.text', 'reached')
    cy.get('[data-testid="tracker-onchain-status"]').should('contain.text', 'finalized')
  })
})
