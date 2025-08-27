#!/usr/bin/env python3
"""
Create a default recipe placeholder image that matches the PicoCSS color scheme
"""

from PIL import Image, ImageDraw, ImageFont
import os

# Create a 400x300 image with a color that matches PicoCSS
# Using a muted blue-gray that works well in both light and dark mode
background_color = '#64748b'  # Slate-500, good for both light/dark
icon_color = '#f1f5f9'       # Slate-100, light text

# Create image
img = Image.new('RGB', (400, 300), background_color)
draw = ImageDraw.Draw(img)

# Draw a simple recipe icon (chef hat or utensils)
# Drawing a simple chef's hat
center_x, center_y = 200, 150

# Hat base (ellipse)
draw.ellipse([center_x-60, center_y-10, center_x+60, center_y+30], fill=icon_color)

# Hat top (circle)
draw.ellipse([center_x-40, center_y-50, center_x+40, center_y+10], fill=icon_color)

# Hat fold line
draw.arc([center_x-55, center_y-5, center_x+55, center_y+25], 0, 180, fill=background_color, width=3)

# Save the image
img.save('/home/sivert/code/htmx-rs-todo/photos/default-recipe.jpg', 'JPEG', quality=85, optimize=True)
print("Default recipe image created at photos/default-recipe.jpg")