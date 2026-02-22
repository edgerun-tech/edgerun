describe('thread benchmark', () => {
  it('benchmarks main thread and worker thread and reports throughput metrics', () => {
    cy.visit('/workers/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)

    cy.get('[data-testid="thread-benchmark-card"]').should('be.visible')
    cy.get('[data-testid="thread-benchmark-iterations"]').clear().type('4')
    cy.get('[data-testid="thread-benchmark-work-scale"]').clear().type('30000')
    cy.get('[data-testid="thread-benchmark-max-payload"]').clear().type('2')

    cy.get('[data-testid="thread-benchmark-run"]').click({ force: true })

    cy.get('[data-testid="thread-benchmark-status"]', { timeout: 60000 }).should('contain', 'completed')
    cy.get('[data-testid="thread-benchmark-error"]').should('contain', 'No benchmark errors reported.')
    cy.get('[data-testid="thread-benchmark-main-total"]').invoke('text').should((text) => {
      expect(String(text || '')).to.match(/\d/)
    })
    cy.get('[data-testid="thread-benchmark-worker-total"]').invoke('text').should((text) => {
      expect(String(text || '')).to.match(/\d/)
    })
    cy.get('[data-testid="thread-benchmark-speedup"]').invoke('text').should((text) => {
      expect(String(text || '')).to.match(/\d/)
    })
    cy.get('[data-testid="thread-benchmark-worker-payload"]').invoke('text').should((text) => {
      expect(String(text || '')).to.match(/MB/)
    })
  })
})
