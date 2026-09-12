const { test, expect } = require('@playwright/test');

const baseUrl = process.env.BASE_URL || 'http://localhost:8787';

test('theme preference persists across reloads', async ({ page }) => {
  await page.goto(baseUrl);
  const toggle = page.getByRole('button', { name: 'Switch theme' });

  await toggle.click();
  await toggle.click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await expect(toggle).toContainText('dark');

  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
});

test('archive filters combine tags with OR semantics', async ({ page }) => {
  await page.goto(`${baseUrl}/blog`);
  await page.getByRole('button', { name: 'show' }).click();

  await page.locator('[data-filter-controls] [data-tag="bitcoin"]').click();
  const bitcoinCount = await page.locator('[data-post-list] .post-row:not([hidden])').count();
  expect(bitcoinCount).toBeGreaterThan(0);

  await page.locator('[data-filter-controls] [data-tag="floresta"]').click();
  const combinedCount = await page.locator('[data-post-list] .post-row:not([hidden])').count();
  expect(combinedCount).toBeGreaterThanOrEqual(bitcoinCount);
});

test('math and sidenotes remain progressively enhanced and navigable', async ({ page }) => {
  await page.goto(`${baseUrl}/blog/floresta-poc`);
  await expect(page.locator('.katex').first()).toBeVisible();
  await expect(page.locator('link[href*="katex.min.css"]')).toHaveCount(1);

  await page.goto(`${baseUrl}/blog/floresta-mainnet-journey`);
  const reference = page.locator('.sidenote-reference').first();
  await expect(reference).toHaveAttribute('href', '#sidenote-1');
  await expect(page.locator('#sidenote-1 .sidenote-back')).toHaveAttribute(
    'href',
    '#sidenote-ref-1',
  );
});
