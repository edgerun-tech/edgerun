// SPDX-License-Identifier: Apache-2.0

describe('intent ui intentbar weather fallback', () => {
  it('falls back to open-meteo when /api/weather is unavailable', () => {
    cy.intercept('GET', '/api/weather*', {
      statusCode: 503,
      body: { ok: false, error: 'weather route unavailable' }
    }).as('weatherRoute')

    cy.intercept('GET', 'https://api.open-meteo.com/v1/forecast*', {
      statusCode: 200,
      body: {
        current: {
          temperature_2m: 21.4,
          relative_humidity_2m: 42,
          apparent_temperature: 20.8,
          wind_speed_10m: 7.2,
          weather_code: 1
        },
        daily: {
          weather_code: [1, 3, 61],
          temperature_2m_max: [23.1, 22.5, 19.2],
          temperature_2m_min: [14.1, 13.7, 11.4]
        }
      }
    }).as('openMeteo')

    cy.visit('/intent-ui/', {
      onBeforeLoad(win) {
        win.localStorage.removeItem('intent-ui-weather-snapshot-v1')
        win.localStorage.setItem('intent-ui-weather-coords', JSON.stringify({
          lat: 52.52,
          lon: 13.405,
          location: 'Berlin, DE'
        }))
      }
    })

    cy.wait('@weatherRoute')
    cy.wait('@openMeteo')
    cy.get('[data-testid="intentbar-weather-temp"]').should('contain.text', '21°')
    cy.get('[data-testid="intentbar-weather-location"]').should('contain.text', 'Berlin, DE')

    cy.get('input[type="text"]').first().type('weather{enter}', { force: true })
    cy.wait('@weatherRoute')
    cy.wait('@openMeteo')
    cy.get('[data-testid="intentbar-weather-temp"]').should('contain.text', '21°')
  })
})
