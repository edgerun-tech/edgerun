describe('run job orchestration UX', () => {
  it('supports preset and custom module flows with clear I/O contract', () => {
    cy.visit('/run/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)

    cy.get('[data-testid="run-step-choose"]').should('be.visible')
    cy.contains('button', '1. Choose Module').click({ force: true })

    cy.get('[aria-label="Preset App"]').should('be.visible')
    cy.get('[data-testid="preset-mode-panel"]').should('be.visible')
    cy.contains('Solana Vanity Address Generator').should('be.visible')

    cy.get('[aria-label="Submission Mode"]').select('Upload Custom Module')
    cy.get('[data-testid="custom-mode-panel"]').should('be.visible')
    cy.get('[aria-label="Custom Module Name"]').should('be.visible')

    cy.get('[aria-label="Submission Mode"]').select('Preset App')
    cy.get('[data-testid="preset-mode-panel"]').should('be.visible')

    cy.contains('button', '2. Define Inputs').click({ force: true })
    cy.get('[data-testid="run-step-inputs"]').should('be.visible')

    cy.get('[aria-label="Input Source"]').select('Raw JSON payload')
    cy.get('[data-testid="json-input-panel"]').should('be.visible')
    cy.get('[aria-label="Input JSON"]').should('contain.value', '"prefix": "So1"')

    cy.get('[aria-label="Input Source"]').select('Upload input file')
    cy.get('[data-testid="file-input-panel"]').should('be.visible')

    cy.get('[aria-label="Input Source"]').select('Predefined fields')
    cy.get('[data-testid="predefined-input-panel"]').should('be.visible')

    cy.get('[data-testid="input-clarity-panel"]').within(() => {
      cy.contains('Input:').should('be.visible')
      cy.contains('Output:').should('be.visible')
      cy.contains('Expected Behavior:').should('be.visible')
    })

    cy.contains('button', '3. Review + Run').click({ force: true })
    cy.get('[data-testid="run-step-review"]').should('be.visible')
    cy.contains('h3', 'Input').should('be.visible')
    cy.contains('h3', 'Output').should('be.visible')
    cy.contains('h3', 'What Will Happen').should('be.visible')
  })
})
