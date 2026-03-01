// SPDX-License-Identifier: Apache-2.0

describe('intent ui calendar panel', () => {
  const seedProfileSession = (win) => {
    win.sessionStorage.setItem('intent-ui-profile-mode-v1', 'profile')
    win.sessionStorage.setItem('intent-ui-profile-id-v1', 'profile_calendar_panel')
    win.sessionStorage.setItem('intent-ui-profile-backend-v1', 'browser_local')
    win.sessionStorage.setItem(
      'intent-ui-profile-scopes-v1',
      JSON.stringify([
        'openid',
        'profile',
        'edgerun:profile.read',
        'edgerun:profile.write',
        'edgerun:intents.submit'
      ])
    )
  }

  it('loads google events and filters by selected day', () => {
    cy.intercept('GET', '/api/google/events*', {
      statusCode: 200,
      body: {
        ok: true,
        items: [
          {
            id: 'evt-1',
            summary: 'Daily Standup',
            start: { dateTime: '2026-03-02T09:00:00.000Z' },
            organizer: { email: 'team@example.com' }
          },
          {
            id: 'evt-2',
            summary: 'Retro',
            start: { dateTime: '2026-03-03T16:00:00.000Z' },
            organizer: { email: 'team@example.com' }
          }
        ]
      }
    }).as('googleEvents')

    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        seedProfileSession(win)
        win.localStorage.setItem('google_token', 'test-google-calendar-token')
      }
    })

    cy.window().then((win) => {
      win.__intentDebug.openWindow('calendar')
    })

    cy.get('[data-testid="calendar-panel"]').should('exist')
    cy.get('[data-testid="calendar-refresh"]').click({ force: true })
    cy.wait('@googleEvents')
    cy.get('[data-testid="calendar-toggle-filter"]').click({ force: true })
    cy.get('[data-testid="calendar-event-item"]').should('have.length.at.least', 2)
    cy.contains('Daily Standup').should('exist')
    cy.contains('Retro').should('exist')
  })
})
