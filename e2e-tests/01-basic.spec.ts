import { test, expect } from '@playwright/test';

test.describe('Basic App Functionality', () => {
  test('homepage loads correctly', async ({ page }) => {
    await page.goto('/');
    
    // Check main elements are present
    await expect(page.locator('h1')).toContainText('HTMX + Rust + SQLite = crappy todo app');
    await expect(page.locator('nav')).toBeVisible();
    
    // Check navigation links
    await expect(page.locator('nav a[href="/"]')).toContainText('Home');
    await expect(page.locator('nav a:has-text("Manage")')).toBeVisible();
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
    await expect(page.locator('h2')).toContainText('Week ');
    
    // Navigate to manage
    await page.click('nav a[href="/manage"]');
    await expect(page).toHaveURL('/manage');
    
    // Navigate back to home
    await page.click('nav a[href="/"]');
    await expect(page).toHaveURL('/');
  });

  test('can create and manage todo lists', async ({ page }) => {
    await page.goto('/manage');
    
    // Create a new list with timestamp to ensure uniqueness
    const testListName = `E2E Test List ${Date.now()}`;
    await page.fill('input[name="name"]', testListName);
    await page.click('button[type="submit"]');
    
    // Verify list was created by checking the dropdown
    await expect(page.locator('select')).toContainText(testListName);
    
    // Navigate back to home and check list is available
    await page.goto('/');
    await expect(page.locator('select')).toContainText(testListName);
  });

  test('can create and toggle tasks', async ({ page }) => {
    await page.goto('/manage');
    
    // Create a test list first with timestamp for uniqueness
    const testListName = `Task Test List ${Date.now()}`;
    await page.fill('input[name="name"]', testListName);
    await page.click('button[type="submit"]');
    
    // Wait for list to be created
    await page.waitForTimeout(500);
    
    // Go to home and select the list
    await page.goto('/');
    await page.selectOption('select', { label: testListName });
    
    // Create a task
    const testTaskName = `E2E Test Task ${Date.now()}`;
    await page.fill('input[placeholder="Add new task"]', testTaskName);
    await page.press('input[placeholder="Add new task"]', 'Enter');
    
    // Wait for task to be created and verify
    await page.waitForTimeout(500);
    await expect(page.locator('text=' + testTaskName)).toBeVisible();
    
    // Toggle task completion
    const checkbox = page.locator('input[type="checkbox"]').first();
    await checkbox.check();
    await page.waitForTimeout(300);
    await expect(checkbox).toBeChecked();
    
    // Toggle back
    await checkbox.uncheck();
    await page.waitForTimeout(300);
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
    await expect(page.locator('a[role="button"].new-recipe-btn')).toContainText('+ New Recipe');
    
    // Check meal plan on mobile
    await page.goto('/meal-plan');
    await expect(page.locator('h2')).toContainText('Week ');
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