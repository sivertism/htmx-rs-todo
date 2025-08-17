import { test, expect } from '@playwright/test';

test.describe('Basic App Functionality', () => {
  test('homepage loads correctly', async ({ page }) => {
    await page.goto('/');
    
    // Check main elements are present
    await expect(page.locator('h1')).toContainText('HTMX + Rust + SQLite = crappy todo app');
    await expect(page.locator('nav')).toBeVisible();
    
    // Check navigation links
    await expect(page.locator('nav a[href="/"]')).toContainText('Home');
    await expect(page.locator('nav a[href="/manage"]')).toContainText('Manage');
    await expect(page.locator('nav a[href="/recipes"]')).toContainText('Recipes');
    await expect(page.locator('nav a[href="/meal-plan"]')).toContainText('Meal Plan');
  });

  test('can navigate between pages', async ({ page }) => {
    await page.goto('/');
    
    // Navigate to recipes
    await page.click('nav a[href="/recipes"]');
    await expect(page).toHaveURL('/recipes');
    await expect(page.locator('h1')).toContainText('Recipes');
    
    // Navigate to meal plan
    await page.click('nav a[href="/meal-plan"]');
    await expect(page).toHaveURL('/meal-plan');
    await expect(page.locator('h1')).toContainText('Meal Plan');
    
    // Navigate to manage
    await page.click('nav a[href="/manage"]');
    await expect(page).toHaveURL('/manage');
    
    // Navigate back to home
    await page.click('nav a[href="/"]');
    await expect(page).toHaveURL('/');
  });

  test('can create and manage todo lists', async ({ page }) => {
    await page.goto('/manage');
    
    // Create a new list
    await page.fill('input[name="name"]', 'E2E Test List');
    await page.click('button[type="submit"]');
    
    // Verify list was created
    await expect(page.locator('text=E2E Test List')).toBeVisible();
    
    // Navigate back to home and check list is available
    await page.goto('/');
    await expect(page.locator('select')).toContainText('E2E Test List');
  });

  test('can create and toggle tasks', async ({ page }) => {
    await page.goto('/manage');
    
    // Create a test list first
    await page.fill('input[name="name"]', 'Task Test List');
    await page.click('button[type="submit"]');
    
    // Go to home and select the list
    await page.goto('/');
    await page.selectOption('select', { label: 'Task Test List' });
    
    // Create a task
    await page.fill('input[placeholder="Add new task"]', 'E2E Test Task');
    await page.click('button[type="submit"]');
    
    // Verify task was created
    await expect(page.locator('text=E2E Test Task')).toBeVisible();
    
    // Toggle task completion
    const checkbox = page.locator('input[type="checkbox"]').first();
    await checkbox.check();
    await expect(checkbox).toBeChecked();
    
    // Toggle back
    await checkbox.uncheck();
    await expect(checkbox).not.toBeChecked();
  });

  test('mobile responsive design works', async ({ page }) => {
    // Test mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');
    
    // Check navigation is mobile-friendly
    await expect(page.locator('nav')).toBeVisible();
    
    // Check recipe page on mobile
    await page.goto('/recipes');
    await expect(page.locator('h1')).toContainText('Recipes');
    await expect(page.locator('a[role="button"]')).toContainText('+ New Recipe');
    
    // Check meal plan on mobile
    await page.goto('/meal-plan');
    await expect(page.locator('h1')).toContainText('Meal Plan');
    await expect(page.locator('.meal-plan-grid')).toBeVisible();
  });

  test('vendor assets load correctly', async ({ page }) => {
    await page.goto('/');
    
    // Check that HTMX is loaded
    const htmxLoaded = await page.evaluate(() => {
      return typeof window.htmx !== 'undefined';
    });
    expect(htmxLoaded).toBe(true);
    
    // Check that CSS is applied (PicoCSS)
    const hasStyles = await page.locator('body').evaluate((el) => {
      const styles = window.getComputedStyle(el);
      return styles.fontFamily !== '';
    });
    expect(hasStyles).toBe(true);
  });
});