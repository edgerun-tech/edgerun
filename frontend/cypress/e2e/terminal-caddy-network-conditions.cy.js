describe('terminal caddy network conditions', () => {
  const caddyBase = 'http://127.0.0.1:9000'

  before(() => {
    cy.exec('../scripts/caddy/set-scenario.sh healthy', { failOnNonZeroExit: true })
  })

  after(() => {
    cy.exec('../scripts/caddy/set-scenario.sh healthy', { failOnNonZeroExit: false })
  })

  it('reflects scheduler outage and recovery when caddy scenario changes', () => {
    cy.visit('/', {
      onBeforeLoad(win) {
        win.localStorage.setItem('edgerun.route.controlBase', caddyBase)
      }
    })

    cy.get('[data-testid="route-debug-scheduler"]', { timeout: 12000 })
      .should('contain.text', 'scheduler online')

    cy.exec('../scripts/caddy/set-scenario.sh scheduler_down', { failOnNonZeroExit: true })

    cy.get('[data-testid="route-debug-scheduler"]', { timeout: 15000 })
      .should('contain.text', 'scheduler offline')

    cy.exec('../scripts/caddy/set-scenario.sh healthy', { failOnNonZeroExit: true })

    cy.get('[data-testid="route-debug-scheduler"]', { timeout: 15000 })
      .should('contain.text', 'scheduler online')
  })
})
