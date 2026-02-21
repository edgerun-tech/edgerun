import { expect, test } from '@playwright/test'

async function expectDialogInViewport(page: import('@playwright/test').Page) {
  const dialog = page.locator('[role="dialog"]').first()
  await expect(dialog).toBeVisible()
  const box = await dialog.boundingBox()
  expect(box).not.toBeNull()
  if (!box) return

  const viewport = page.viewportSize()
  expect(viewport).not.toBeNull()
  if (!viewport) return

  expect(box.x).toBeGreaterThanOrEqual(0)
  expect(box.y).toBeGreaterThanOrEqual(0)
  expect(box.x + box.width).toBeLessThanOrEqual(viewport.width)
  expect(box.y + box.height).toBeLessThanOrEqual(viewport.height)
}

test('wallet dialog stays inside viewport bounds', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 })
  await page.goto('/')
  await page.getByRole('button', { name: /wallet/i }).first().click()
  await expectDialogInViewport(page)
})

test('run mode-safety dialog stays inside viewport bounds', async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 640 })
  await page.goto('/run/')
  await page.getByRole('button', { name: /mode safety/i }).click()
  await expectDialogInViewport(page)
})
