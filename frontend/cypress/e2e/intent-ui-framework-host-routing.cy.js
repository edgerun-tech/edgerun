// SPDX-License-Identifier: Apache-2.0

describe('intent ui framework host routing', () => {
  it('serves intent-ui shell at framework host root', () => {
    cy.request({
      url: 'http://127.0.0.1:4175/',
      headers: {
        Host: 'framework.bengal-salary.ts.net'
      }
    }).then((response) => {
      expect(response.status).to.eq(200)
      expect(response.headers['content-type']).to.include('text/html')
      expect(response.body).to.include('<title>Intent UI</title>')
      expect(response.body).to.include('<main id="root"></main>')
      expect(response.body).to.include('/intent-ui/client/main.js')
    })
  })
})
