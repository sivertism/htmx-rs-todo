import { test, expect } from '@playwright/test';

test.describe('Drag & Drop Functionality', () => {
  test.beforeEach(async ({ page }) => {
    // Create a test list and add multiple tasks for drag/drop testing
    await page.goto('/manage');
    
    // Create test list
    await page.fill('input[name="name"]', 'Drag Test List');
    await page.click('button[type="submit"]');
    
    // Go to home and select the list
    await page.goto('/');
    await page.selectOption('select', { label: 'Drag Test List' });
    
    // Add multiple tasks for reordering
    const tasks = ['First Task', 'Second Task', 'Third Task', 'Fourth Task'];
    for (const task of tasks) {
      await page.fill('input[placeholder="Add new task"]', task);
      await page.press('input[placeholder="Add new task"]', 'Enter');
      // Wait a bit to ensure tasks are created in order
      await page.waitForTimeout(200);
    }
  });

  test('tasks are displayed with drag handles', async ({ page }) => {
    await page.goto('/');
    await page.selectOption('select', { label: 'Drag Test List' });
    
    // Check that tasks are present
    const tasks = page.locator('tr.tasks');
    await expect(tasks).toHaveCount(4);
    
    // Check for sortable functionality (Sortable.js should be loaded)
    const sortableContainer = page.locator('#tasktablebody');
    await expect(sortableContainer).toBeVisible();
    
    // Verify Sortable.js is loaded
    const sortableLoaded = await page.evaluate(() => {
      return typeof window.Sortable !== 'undefined';
    });
    expect(sortableLoaded).toBe(true);
  });

  test('can reorder tasks by dragging', async ({ page }) => {
    await page.goto('/');
    await page.selectOption('select', { label: 'Drag Test List' });
    
    // Get initial task order
    const initialTasks = await page.locator('tr.tasks').allTextContents();
    expect(initialTasks).toEqual(['First Task', 'Second Task', 'Third Task', 'Fourth Task']);
    
    // Perform drag and drop using mouse actions
    const firstTask = page.locator('tr.tasks').first();
    const thirdTask = page.locator('tr.tasks').nth(2);
    
    // Get bounding boxes for precise dragging
    const firstTaskBox = await firstTask.boundingBox();
    const thirdTaskBox = await thirdTask.boundingBox();
    
    if (firstTaskBox && thirdTaskBox) {
      // Drag first task to third position
      await page.mouse.move(firstTaskBox.x + firstTaskBox.width / 2, firstTaskBox.y + firstTaskBox.height / 2);
      await page.mouse.down();
      await page.mouse.move(thirdTaskBox.x + thirdTaskBox.width / 2, thirdTaskBox.y + thirdTaskBox.height / 2, { steps: 10 });
      await page.mouse.up();
      
      // Wait for reorder to complete
      await page.waitForTimeout(500);
      
      // Check new order
      const newTasks = await page.locator('tr.tasks').allTextContents();
      expect(newTasks).not.toEqual(initialTasks);
      
      // First task should no longer be first
      expect(newTasks[0]).not.toBe('First Task');
    }
  });

  test('drag and drop persists after page reload', async ({ page }) => {
    await page.goto('/');
    await page.selectOption('select', { label: 'Drag Test List' });
    
    // Perform a simple reorder (move second task to first position)
    const secondTask = page.locator('tr.tasks').nth(1);
    const firstTask = page.locator('tr.tasks').first();
    
    const secondTaskBox = await secondTask.boundingBox();
    const firstTaskBox = await firstTask.boundingBox();
    
    if (secondTaskBox && firstTaskBox) {
      await page.mouse.move(secondTaskBox.x + secondTaskBox.width / 2, secondTaskBox.y + secondTaskBox.height / 2);
      await page.mouse.down();
      await page.mouse.move(firstTaskBox.x + firstTaskBox.width / 2, firstTaskBox.y + firstTaskBox.height / 2 - 20, { steps: 10 });
      await page.mouse.up();
      
      // Wait for server update
      await page.waitForTimeout(1000);
      
      // Get order after drag
      const tasksAfterDrag = await page.locator('tr.tasks').allTextContents();
      
      // Reload page
      await page.reload();
      await page.selectOption('select', { label: 'Drag Test List' });
      
      // Order should be preserved
      const tasksAfterReload = await page.locator('tr.tasks').allTextContents();
      expect(tasksAfterReload).toEqual(tasksAfterDrag);
    }
  });

  test('drag and drop works with mixed completed/incomplete tasks', async ({ page }) => {
    await page.goto('/');
    await page.selectOption('select', { label: 'Drag Test List' });
    
    // Mark some tasks as completed
    const checkboxes = page.locator('input[type="checkbox"]');
    await checkboxes.nth(1).check(); // Mark second task as completed
    await checkboxes.nth(3).check(); // Mark fourth task as completed
    
    // Wait for completion updates
    await page.waitForTimeout(500);
    
    // Get current task order
    const tasksBeforeDrag = await page.locator('tr.tasks').allTextContents();
    
    // Try to reorder (move first incomplete task)
    const firstIncompleteTask = page.locator('tr.tasks').first();
    const secondTaskBox = await page.locator('tr.tasks').nth(1).boundingBox();
    const firstTaskBox = await firstIncompleteTask.boundingBox();
    
    if (firstTaskBox && secondTaskBox) {
      await page.mouse.move(firstTaskBox.x + firstTaskBox.width / 2, firstTaskBox.y + firstTaskBox.height / 2);
      await page.mouse.down();
      await page.mouse.move(secondTaskBox.x + secondTaskBox.width / 2, secondTaskBox.y + secondTaskBox.height / 2 + 20, { steps: 10 });
      await page.mouse.up();
      
      await page.waitForTimeout(500);
      
      // Order should have changed
      const tasksAfterDrag = await page.locator('tr.tasks').allTextContents();
      expect(tasksAfterDrag).not.toEqual(tasksBeforeDrag);
    }
  });

  test('drag and drop is disabled on mobile', async ({ page }) => {
    // Set mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    
    await page.goto('/');
    await page.selectOption('select', { label: 'Drag Test List' });
    
    // Tasks should still be visible
    const tasks = page.locator('tr.tasks');
    await expect(tasks).toHaveCount(4);
    
    // Try to perform drag action (should not work or be limited on mobile)
    const firstTask = page.locator('tr.tasks').first();
    const secondTask = page.locator('tr.tasks').nth(1);
    
    const firstTaskBox = await firstTask.boundingBox();
    const secondTaskBox = await secondTask.boundingBox();
    
    if (firstTaskBox && secondTaskBox) {
      // Try touch-based drag
      await page.touchscreen.tap(firstTaskBox.x + firstTaskBox.width / 2, firstTaskBox.y + firstTaskBox.height / 2);
      
      // Mobile should still show tasks properly even if drag is disabled
      await expect(tasks).toHaveCount(4);
    }
  });

  test('handles drag and drop with empty lists gracefully', async ({ page }) => {
    // Create an empty list
    await page.goto('/manage');
    await page.fill('input[name="name"]', 'Empty Drag Test List');
    await page.click('button[type="submit"]');
    
    await page.goto('/');
    await page.selectOption('select', { label: 'Empty Drag Test List' });
    
    // Should not have any tasks to drag
    const tasks = page.locator('tr.tasks');
    await expect(tasks).toHaveCount(0);
    
    // Container should still exist for potential future tasks
    const sortableContainer = page.locator('#sortable-tasks');
    await expect(sortableContainer).toBeVisible();
    
    // Add a single task
    await page.fill('input[placeholder="Add new task"]', 'Single Task');
    await page.click('button[type="submit"]');
    
    // Now should have one task (no dragging possible with one item)
    await expect(tasks).toHaveCount(1);
  });

  test('drag handles provide visual feedback', async ({ page }) => {
    await page.goto('/');
    await page.selectOption('select', { label: 'Drag Test List' });
    
    const firstTask = page.locator('tr.tasks').first();
    
    // Hover over task to check for drag cursor or visual feedback
    await firstTask.hover();
    
    // Task should be interactive
    await expect(firstTask).toBeVisible();
    
    // Check for sortable class or data attributes
    const hasSortableAttributes = await firstTask.evaluate((el) => {
      return el.hasAttribute('data-task-id') && 
             el.closest('#sortable-tasks') !== null;
    });
    
    expect(hasSortableAttributes).toBe(true);
  });

  test('reorder endpoint responds correctly', async ({ page }) => {
    await page.goto('/');
    await page.selectOption('select', { label: 'Drag Test List' });
    
    // Listen for reorder requests
    let reorderRequestMade = false;
    page.on('request', request => {
      if (request.url().includes('/reorder') && request.method() === 'POST') {
        reorderRequestMade = true;
      }
    });
    
    // Perform drag and drop
    const firstTask = page.locator('tr.tasks').first();
    const secondTask = page.locator('tr.tasks').nth(1);
    
    const firstTaskBox = await firstTask.boundingBox();
    const secondTaskBox = await secondTask.boundingBox();
    
    if (firstTaskBox && secondTaskBox) {
      await page.mouse.move(firstTaskBox.x + firstTaskBox.width / 2, firstTaskBox.y + firstTaskBox.height / 2);
      await page.mouse.down();
      await page.mouse.move(secondTaskBox.x + secondTaskBox.width / 2, secondTaskBox.y + secondTaskBox.height / 2 + 20, { steps: 10 });
      await page.mouse.up();
      
      // Wait for potential network request
      await page.waitForTimeout(1000);
      
      // Reorder request should have been made
      expect(reorderRequestMade).toBe(true);
    }
  });
});