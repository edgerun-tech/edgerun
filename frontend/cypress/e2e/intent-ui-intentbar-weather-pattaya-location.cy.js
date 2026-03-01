// SPDX-License-Identifier: Apache-2.0

describe('intent ui intentbar weather pattaya location', () => {
  it('uses Pattaya coordinates and surfaces Pattaya location label', () => {
    cy.intercept('GET', '/api/weather*', (req) => {
      const url = new URL(req.url)
      expect(url.searchParams.get('lat')).to.eq('12.9236')
      expect(url.searchParams.get('lon')).to.eq('100.8825')
      req.reply({
        statusCode: 200,
        body: {
          ok: true,
          weather: {
            temp: 31,
            condition: 'sunny',
            humidity: 68,
            windSpeed: 14,
            feelsLike: 35,
            location: '',
            forecast: []
          }
        }
      })
    }).as('weather')

    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        win.localStorage.removeItem('intent-ui-weather-snapshot-v1')
        win.localStorage.removeItem('intent-ui-weather-coords')
      }
    })

    cy.wait('@weather')
    cy.get('[data-testid="intentbar-weather-temp"]').should('contain.text', '31°')
    cy.get('[data-testid="intentbar-weather-location"]').should('contain.text', 'Pattaya, Thailand')
  })
})
