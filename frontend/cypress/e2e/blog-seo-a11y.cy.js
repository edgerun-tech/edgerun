// SPDX-License-Identifier: Apache-2.0
describe('blog seo and a11y baseline', () => {
  it('renders readable blog index without placeholder generating cards', () => {
    cy.visit('/blog/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)
    cy.contains('h1', 'Blog').should('be.visible')
    cy.contains('Read: Why Edgerun Exists').should('be.visible')
    cy.get('main').within(() => {
      cy.contains('Generating').should('not.exist')
    })
  })

  it('renders long-form why post and exposes canonical/og metadata', () => {
    cy.visit('/blog/introducing-edgerun/')
    cy.window().its('__EDGERUN_HYDRATED').should('eq', true)
    cy.contains('h1', 'Introducing Edgerun').should('be.visible')
    cy.get('[data-testid="blog-article"] h2').should('have.length.at.least', 3)

    cy.get('head link[rel="canonical"]').should('have.attr', 'href').and('include', '/blog/introducing-edgerun/')
    cy.get('head meta[property="og:title"]').should('have.attr', 'content').and('include', 'Introducing Edgerun')
    cy.get('head meta[property="og:description"]').should('have.attr', 'content').and('not.be.empty')
    cy.get('head meta[name="twitter:card"]').should('have.attr', 'content', 'summary_large_image')
  })
})
