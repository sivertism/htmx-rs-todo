import { test, expect } from '@playwright/test';
import { promises as fs } from 'fs';
import path from 'path';

test.describe('Recipe Management', () => {
  test('can access recipes page and see existing recipes', async ({ page }) => {
    await page.goto('/recipes');
    
    // Check page structure
    await expect(page.locator('h1')).toContainText('Recipes');
    await expect(page.locator('a[role="button"]')).toContainText('+ New Recipe');
    
    // Check recipe cards are displayed (assuming some exist)
    const recipeCards = page.locator('.recipe-card');
    const cardCount = await recipeCards.count();
    
    if (cardCount > 0) {
      // Test first recipe card has expected elements
      const firstCard = recipeCards.first();
      await expect(firstCard.locator('h3')).toBeVisible();
      await expect(firstCard.locator('.recipe-actions a')).toContainText('+ Add to List');
    }
  });

  test('can create a new recipe without photo', async ({ page }) => {
    await page.goto('/recipes/new');
    
    // Fill recipe form
    await page.fill('input[name="title"]', 'E2E Test Recipe');
    await page.fill('textarea[name="ingredients"]', 'Test ingredient 1\nTest ingredient 2\nTest ingredient 3');
    await page.fill('textarea[name="instructions"]', 'Mix all ingredients together. Cook until done. Visit https://example.com for more tips.');
    
    // Submit form
    await page.click('button[type="submit"]');
    
    // Should redirect to recipes page
    await expect(page).toHaveURL('/recipes');
    
    // Recipe should appear in list
    await expect(page.locator('text=E2E Test Recipe')).toBeVisible();
  });

  test('can view recipe detail page', async ({ page }) => {
    await page.goto('/recipes');
    
    // Click on a recipe (assuming at least one exists from previous test)
    const recipeCard = page.locator('.recipe-card').first();
    const recipeTitle = await recipeCard.locator('h3').textContent();
    
    await recipeCard.locator('a').first().click();
    
    // Should be on recipe detail page
    await expect(page.locator('h1')).toContainText(recipeTitle || '');
    
    // Check detail page elements
    await expect(page.locator('.recipe-ingredients')).toBeVisible();
    await expect(page.locator('.recipe-instructions')).toBeVisible();
    await expect(page.locator('a[role="button"]')).toContainText('+ Add to List');
    
    // Check for edit/delete buttons
    await expect(page.locator('a[href*="/edit"]')).toBeVisible();
    await expect(page.locator('button')).toContainText('Delete Recipe');
  });

  test('recipe instructions auto-link URLs', async ({ page }) => {
    await page.goto('/recipes/new');
    
    // Create recipe with URL in instructions
    await page.fill('input[name="title"]', 'Recipe with Links');
    await page.fill('textarea[name="ingredients"]', 'Test ingredient');
    await page.fill('textarea[name="instructions"]', 'Visit https://example.com and http://test.org for more info.');
    
    await page.click('button[type="submit"]');
    
    // View the recipe
    await page.click('text=Recipe with Links');
    
    // URLs should be clickable links
    const links = page.locator('.recipe-instructions a');
    await expect(links).toHaveCount(2);
    await expect(links.first()).toHaveAttribute('href', 'https://example.com');
    await expect(links.nth(1)).toHaveAttribute('href', 'http://test.org');
    await expect(links.first()).toHaveAttribute('target', '_blank');
  });

  test('can edit existing recipe', async ({ page }) => {
    await page.goto('/recipes');
    
    // Click on first recipe
    const firstRecipe = page.locator('.recipe-card').first();
    await firstRecipe.locator('a').first().click();
    
    // Click edit button
    await page.click('a[href*="/edit"]');
    
    // Should be on edit page
    await expect(page).toHaveURL(/\/recipes\/\d+\/edit/);
    
    // Modify title
    const titleInput = page.locator('input[name="title"]');
    await titleInput.fill('Updated Recipe Title');
    
    // Submit changes
    await page.click('button[type="submit"]');
    
    // Should redirect back to recipe detail
    await expect(page.locator('h1')).toContainText('Updated Recipe Title');
  });

  test('can add recipe to todo list', async ({ page }) => {
    // First ensure there's at least one todo list
    await page.goto('/manage');
    await page.fill('input[name="name"]', 'Recipe Test List');
    await page.click('button[type="submit"]');
    
    // Go to recipes and add one to list
    await page.goto('/recipes');
    const firstRecipe = page.locator('.recipe-card').first();
    await firstRecipe.locator('a[role="button"]').click();
    
    // Should be on add-to-list form
    await expect(page).toHaveURL(/\/recipes\/\d+\/add-to-list/);
    await expect(page.locator('h1')).toContainText('Add Recipe to List');
    
    // Select the list
    await page.selectOption('select[name="list_id"]', { label: 'Recipe Test List' });
    
    // Should have ingredient checkboxes (all checked by default)
    const checkboxes = page.locator('input[type="checkbox"][name="ingredients[]"]');
    const checkboxCount = await checkboxes.count();
    expect(checkboxCount).toBeGreaterThan(0);
    
    // All should be checked by default
    for (let i = 0; i < checkboxCount; i++) {
      await expect(checkboxes.nth(i)).toBeChecked();
    }
    
    // Test select all / deselect all
    await page.click('button:has-text("Deselect All")');
    for (let i = 0; i < checkboxCount; i++) {
      await expect(checkboxes.nth(i)).not.toBeChecked();
    }
    
    await page.click('button:has-text("Select All")');
    for (let i = 0; i < checkboxCount; i++) {
      await expect(checkboxes.nth(i)).toBeChecked();
    }
    
    // Submit form
    await page.click('button[type="submit"]:has-text("Add to List")');
    
    // Should redirect to todo list
    await expect(page).toHaveURL('/');
  });

  test('photo upload functionality works', async ({ page }) => {
    await page.goto('/recipes/new');
    
    // Fill basic recipe info
    await page.fill('input[name="title"]', 'Recipe with Photo');
    await page.fill('textarea[name="ingredients"]', 'Photo test ingredient');
    await page.fill('textarea[name="instructions"]', 'Photo test instructions');
    
    // Submit recipe first
    await page.click('button[type="submit"]');
    
    // Navigate to the recipe detail page
    await page.click('text=Recipe with Photo');
    
    // Create a test image file
    const testImagePath = path.join(process.cwd(), 'test-image.jpg');
    const testImageData = Buffer.from('test image data');
    await fs.writeFile(testImagePath, testImageData);
    
    try {
      // Upload photo (should auto-submit due to onchange)
      const fileInput = page.locator('input[type="file"]');
      await fileInput.setInputFiles(testImagePath);
      
      // Wait for form submission and page reload
      await page.waitForLoadState('networkidle');
      
      // Check that photo was uploaded (page should show photo)
      await expect(page.locator('.recipe-photos')).toBeVisible();
    } finally {
      // Clean up test file
      await fs.unlink(testImagePath).catch(() => {});
    }
  });

  test('can delete recipe', async ({ page }) => {
    // Create a recipe to delete
    await page.goto('/recipes/new');
    await page.fill('input[name="title"]', 'Recipe to Delete');
    await page.fill('textarea[name="ingredients"]', 'Delete test ingredient');
    await page.fill('textarea[name="instructions"]', 'Delete test instructions');
    await page.click('button[type="submit"]');
    
    // Navigate to recipe detail
    await page.click('text=Recipe to Delete');
    
    // Click delete button and confirm
    page.on('dialog', dialog => dialog.accept());
    await page.click('button:has-text("Delete Recipe")');
    
    // Should redirect to recipes list
    await expect(page).toHaveURL('/recipes');
    
    // Recipe should no longer exist
    await expect(page.locator('text=Recipe to Delete')).not.toBeVisible();
  });

  test('mobile recipe view is responsive', async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    
    await page.goto('/recipes');
    
    // Check mobile layout
    await expect(page.locator('h1')).toBeVisible();
    await expect(page.locator('a[role="button"]')).toContainText('+ New Recipe');
    
    // Recipe cards should stack properly on mobile
    const recipeCards = page.locator('.recipe-card');
    const cardCount = await recipeCards.count();
    
    if (cardCount > 0) {
      // Cards should be visible and properly sized
      for (let i = 0; i < Math.min(cardCount, 3); i++) {
        await expect(recipeCards.nth(i)).toBeVisible();
      }
    }
    
    // Test new recipe form on mobile
    await page.goto('/recipes/new');
    await expect(page.locator('input[name="title"]')).toBeVisible();
    await expect(page.locator('textarea[name="ingredients"]')).toBeVisible();
    await expect(page.locator('textarea[name="instructions"]')).toBeVisible();
  });
});