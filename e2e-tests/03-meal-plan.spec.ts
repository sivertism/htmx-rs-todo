import { test, expect } from '@playwright/test';

test.describe('Meal Planning', () => {
  test('can access meal plan page and see weekly layout', async ({ page }) => {
    await page.goto('/meal-plan');
    
    // Check page structure
    await expect(page.locator('h1')).toContainText('Meal Plan');
    
    // Should show week navigation
    await expect(page.locator('a:has-text("← Previous Week")')).toBeVisible();
    await expect(page.locator('a:has-text("Next Week →")')).toBeVisible();
    
    // Should show 7-day grid
    await expect(page.locator('.meal-plan-grid')).toBeVisible();
    
    // Should have days of the week
    const dayHeaders = page.locator('.day-header');
    await expect(dayHeaders).toHaveCount(7);
    
    // Check some expected day names
    await expect(page.locator('text=Monday')).toBeVisible();
    await expect(page.locator('text=Tuesday')).toBeVisible();
    await expect(page.locator('text=Sunday')).toBeVisible();
  });

  test('can navigate between weeks', async ({ page }) => {
    await page.goto('/meal-plan');
    
    // Get current week info
    const currentWeekText = await page.locator('.week-info').textContent();
    
    // Navigate to next week
    await page.click('a:has-text("Next Week →")');
    await expect(page).toHaveURL(/\/meal-plan\?week=\d{4}-\d{2}-\d{2}/);
    
    // Week should have changed
    const nextWeekText = await page.locator('.week-info').textContent();
    expect(nextWeekText).not.toBe(currentWeekText);
    
    // Navigate back to previous week
    await page.click('a:has-text("← Previous Week")');
    
    // Should be back to original week
    const backToOriginalText = await page.locator('.week-info').textContent();
    expect(backToOriginalText).toBe(currentWeekText);
  });

  test('can add free-form meal to a day', async ({ page }) => {
    await page.goto('/meal-plan');
    
    // Click "Add Meal" for first day
    const firstAddButton = page.locator('a[role="button"]:has-text("Add Meal")').first();
    await firstAddButton.click();
    
    // Should be on add meal form
    await expect(page).toHaveURL(/\/meal-plan\/\d{4}-\d{2}-\d{2}\/add/);
    await expect(page.locator('h1')).toContainText('Add Meal');
    
    // Fill in free-form meal with unique text
    const mealText = `Spaghetti Bolognese ${Date.now()}`;
    await page.fill('textarea[name="meal_text"]', mealText);
    
    // Submit form and wait for redirect
    await Promise.all([
      page.waitForURL(/\/meal-plan$/, { timeout: 10000 }),
      page.click('button[type="submit"]')
    ]);
    
    // Wait for page to load and meal to appear
    await page.waitForLoadState('networkidle');
    await expect(page.locator(`text=${mealText}`)).toBeVisible();
  });

  test('can add recipe to meal plan', async ({ page }) => {
    // First create a recipe with unique name
    const recipeTitle = `Meal Plan Test Recipe ${Date.now()}`;
    await page.goto('/recipes/new');
    await page.fill('input[name="title"]', recipeTitle);
    await page.fill('textarea[name="ingredients"]', 'Test ingredients for meal plan');
    await page.fill('textarea[name="instructions"]', 'Test instructions for meal plan');
    await page.click('button[type="submit"]');
    
    // Now go to meal plan and add this recipe
    await page.goto('/meal-plan');
    
    // Click "Add Meal" for first day
    const firstAddButton = page.locator('a[role="button"]:has-text("Add Meal")').first();
    await firstAddButton.click();
    
    // Select recipe from dropdown
    const recipeSelect = page.locator('select[name="recipe_id"]');
    await recipeSelect.selectOption({ label: recipeTitle });
    
    // Meal text should auto-populate
    const mealTextArea = page.locator('textarea[name="meal_text"]');
    await expect(mealTextArea).toHaveValue(recipeTitle);
    
    // Submit form
    await page.click('button[type="submit"]');
    
    // Should redirect back to meal plan
    await expect(page).toHaveURL(/\/meal-plan/);
    
    // Recipe should appear in the day
    await expect(page.locator(`text=${recipeTitle}`)).toBeVisible();
  });

  test.skip('can edit existing meal plan entry', async ({ page }) => {
    await page.goto('/meal-plan');
    
    // Add a meal first
    const firstAddButton = page.locator('a[role="button"]:has-text("Add Meal")').first();
    await firstAddButton.click();
    
    await page.fill('textarea[name="meal_text"]', 'Original Meal');
    await page.click('button[type="submit"]');
    
    // Find and click edit button for the meal
    const editButton = page.locator('a:has-text("Edit")').first();
    await editButton.click();
    
    // Should be on edit form
    await expect(page).toHaveURL(/\/meal-plan\/\d{4}-\d{2}-\d{2}\/\d+\/edit/);
    
    // Change meal text
    const mealTextArea = page.locator('textarea[name="meal_text"]');
    await expect(mealTextArea).toHaveValue('Original Meal');
    await mealTextArea.fill('Updated Meal');
    
    // Submit changes
    await page.click('button[type="submit"]');
    
    // Should redirect back to meal plan
    await expect(page).toHaveURL(/\/meal-plan/);
    
    // Updated meal should appear
    await expect(page.locator('text=Updated Meal')).toBeVisible();
    await expect(page.locator('text=Original Meal')).not.toBeVisible();
  });

  test('can delete meal plan entry', async ({ page }) => {
    await page.goto('/meal-plan');
    
    // Add a meal first with unique text
    const mealText = `Meal to Delete ${Date.now()}`;
    const firstAddButton = page.locator('a[role="button"]:has-text("Add Meal")').first();
    await firstAddButton.click();
    
    await page.fill('textarea[name="meal_text"]', mealText);
    await Promise.all([
      page.waitForURL(/\/meal-plan$/, { timeout: 10000 }),
      page.click('button[type="submit"]')
    ]);
    
    // Find and click delete button (trash emoji)
    page.on('dialog', dialog => dialog.accept());
    const deleteButton = page.locator('button:has-text("🗑️")').first();
    await deleteButton.click();
    
    // Meal should be removed
    await expect(page.locator(`text=${mealText}`)).not.toBeVisible();
  });

  test('displays current week by default', async ({ page }) => {
    await page.goto('/meal-plan');
    
    // Should show current week (no specific date in URL)
    await expect(page).toHaveURL('/meal-plan');
    
    // Week info should be displayed
    await expect(page.locator('.week-info')).toBeVisible();
    
    // Should show today's date highlighted or current week range
    const weekInfo = await page.locator('.week-info').textContent();
    expect(weekInfo).toMatch(/\d{4}/); // Should contain year
  });

  test('meal plan is mobile responsive', async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    
    await page.goto('/meal-plan');
    
    // Check mobile layout
    await expect(page.locator('h1')).toBeVisible();
    await expect(page.locator('.meal-plan-grid')).toBeVisible();
    
    // Navigation should work on mobile
    await expect(page.locator('a:has-text("← Previous Week")')).toBeVisible();
    await expect(page.locator('a:has-text("Next Week →")')).toBeVisible();
    
    // Day cards should stack properly on mobile
    const dayCards = page.locator('.day-card');
    const cardCount = await dayCards.count();
    
    if (cardCount > 0) {
      // Cards should be visible and properly sized
      for (let i = 0; i < Math.min(cardCount, 7); i++) {
        await expect(dayCards.nth(i)).toBeVisible();
      }
    }
    
    // Test add meal form on mobile
    const firstAddButton = page.locator('a[role="button"]:has-text("Add Meal")').first();
    await firstAddButton.click();
    
    await expect(page.locator('textarea[name="meal_text"]')).toBeVisible();
    await expect(page.locator('select[name="recipe_id"]')).toBeVisible();
  });

  test('handles empty meal plan gracefully', async ({ page }) => {
    // Navigate to a future week that should be empty
    await page.goto('/meal-plan?week=2030-01-01');
    
    // Should still show proper layout
    await expect(page.locator('h1')).toContainText('Meal Plan');
    await expect(page.locator('.meal-plan-grid')).toBeVisible();
    
    // All days should have "Add Meal" buttons
    const addMealButtons = page.locator('a[role="button"]:has-text("Add Meal")');
    await expect(addMealButtons).toHaveCount(7);
    
    // No meals should be displayed
    const mealEntries = page.locator('.meal-entry');
    await expect(mealEntries).toHaveCount(0);
  });

  test('recipe integration works in meal plan', async ({ page }) => {
    // Create a recipe with unique name
    const recipeTitle = `Integration Test Recipe ${Date.now()}`;
    await page.goto('/recipes/new');
    await page.fill('input[name="title"]', recipeTitle);
    await page.fill('textarea[name="ingredients"]', 'Integration test ingredients');
    await page.fill('textarea[name="instructions"]', 'Integration test instructions');
    await page.click('button[type="submit"]');
    
    // Go to meal plan and add this recipe
    await page.goto('/meal-plan');
    
    const firstAddButton = page.locator('a[role="button"]:has-text("Add Meal")').first();
    await firstAddButton.click();
    
    // Recipe should be available in dropdown
    const recipeSelect = page.locator('select[name="recipe_id"]');
    await expect(recipeSelect).toBeVisible();
    await expect(recipeSelect.locator(`option:has-text("${recipeTitle}")`)).toBeAttached();
    
    // Select the recipe
    await recipeSelect.selectOption({ label: recipeTitle });
    
    // Submit and wait for redirect
    await Promise.all([
      page.waitForURL(/\/meal-plan$/, { timeout: 10000 }),
      page.click('button[type="submit"]')
    ]);
    
    // Wait for page to load and recipe to appear
    await page.waitForLoadState('networkidle');
    await expect(page.locator(`text=${recipeTitle}`)).toBeVisible();
    
    // Should be able to click on recipe to view details
    await page.click(`text=${recipeTitle}`);
    
    // Should navigate to recipe detail page
    await expect(page).toHaveURL(/\/recipes\/\d+/);
    await expect(page.locator('h1')).toContainText(recipeTitle);
  });
});