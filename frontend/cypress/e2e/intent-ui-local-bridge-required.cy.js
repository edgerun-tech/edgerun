// SPDX-License-Identifier: Apache-2.0

describe("intent ui local bridge required", () => {
  it("shows blocking error when local bridge is unavailable", () => {
    cy.visit("/intent-ui/");
    cy.contains("Local Bridge Required").should("be.visible");
    cy.contains("Can't connect to local bridge, is it running?").should("be.visible");
    cy.get("[data-testid='retry-local-bridge']").should("be.visible");
  });
});
