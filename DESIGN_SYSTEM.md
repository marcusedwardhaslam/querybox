# QueryBox Design System

A comprehensive design system for QueryBox - a native-looking Electron SQL GUI application.

## Overview

This design system provides a complete set of UI components, styling utilities, and design tokens that create a native application experience across Windows, macOS, and Linux platforms.

## Features

- 🎨 **Dual Theme Support**: Automatic dark/light mode with system preference detection
- 🖥️ **Native Look & Feel**: System fonts and native-inspired styling
- 📦 **Modular Components**: Reusable SCSS components
- 🎯 **TypeScript Support**: Full type safety throughout
- ⚡ **Performance**: Optimized CSS with minimal runtime overhead
- ♿ **Accessible**: WCAG-compliant color contrasts and keyboard navigation

## Design Tokens

### Colors

The design system uses CSS custom properties for theme switching:

#### Light Mode
- Background: `--bg-primary`, `--bg-secondary`, `--bg-tertiary`
- Text: `--text-primary`, `--text-secondary`, `--text-tertiary`
- Borders: `--border-primary`, `--border-secondary`
- Accent: `--accent-primary`, `--accent-hover`, `--accent-active`

#### Dark Mode
Colors automatically switch based on `[data-theme='dark']` attribute or system preference.

### Typography

System fonts for native appearance:
```scss
font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', ...
```

**Font Sizes**: `xs` (12px) → `4xl` (36px)
**Font Weights**: 300 (light) → 700 (bold)
**Line Heights**: tight (1.25), normal (1.5), relaxed (1.75)

### Spacing

4px base unit with consistent scale:
- `spacing-1` (4px) → `spacing-32` (128px)
- Utility classes: `p-4`, `m-2`, `gap-3`, etc.

### Border Radius

- `radius-sm` (2px) → `radius-2xl` (16px)
- `radius-full` for circular elements

### Shadows

Context-aware shadows that adapt to theme:
- `shadow-sm` → `shadow-xl`
- Automatically intensified in dark mode

## Components

### Buttons

```html
<!-- Variants -->
<button class="btn btn-primary">Primary</button>
<button class="btn btn-secondary">Secondary</button>
<button class="btn btn-outline">Outline</button>
<button class="btn btn-ghost">Ghost</button>
<button class="btn btn-success">Success</button>
<button class="btn btn-danger">Danger</button>

<!-- Sizes -->
<button class="btn btn-primary btn-sm">Small</button>
<button class="btn btn-primary">Default</button>
<button class="btn btn-primary btn-lg">Large</button>

<!-- States -->
<button class="btn btn-primary" disabled>Disabled</button>

<!-- Button Group -->
<div class="btn-group">
  <button class="btn btn-secondary">Left</button>
  <button class="btn btn-secondary">Middle</button>
  <button class="btn btn-secondary">Right</button>
</div>
```

### Input Fields

```html
<!-- Text Input -->
<div class="input-group">
  <label class="input-label required">Username</label>
  <input type="text" class="input" placeholder="Enter username" />
  <span class="input-hint">Helper text goes here</span>
</div>

<!-- Textarea -->
<textarea class="textarea" placeholder="Description"></textarea>

<!-- Sizes -->
<input class="input input-sm" />
<input class="input" />
<input class="input input-lg" />

<!-- States -->
<input class="input input-error" />
<input class="input" disabled />
```

### Checkboxes & Radio Buttons

```html
<!-- Checkbox -->
<label class="checkbox">
  <input type="checkbox" checked />
  <span class="checkbox-label">Accept terms</span>
</label>

<!-- Radio -->
<label class="radio">
  <input type="radio" name="option" checked />
  <span class="radio-label">Option 1</span>
</label>
```

### Select & Dropdowns

```html
<!-- Select Box -->
<div class="select">
  <select>
    <option>Option 1</option>
    <option>Option 2</option>
  </select>
</div>

<!-- Custom Dropdown -->
<div class="dropdown">
  <button class="btn btn-secondary dropdown-trigger">Menu</button>
  <div class="dropdown-menu">
    <div class="dropdown-header">Actions</div>
    <div class="dropdown-item">Edit</div>
    <div class="dropdown-divider"></div>
    <div class="dropdown-item">Delete</div>
  </div>
</div>
```

### Tables

```html
<div class="table-container">
  <table class="table table-striped">
    <thead>
      <tr>
        <th class="sortable">Name</th>
        <th class="sortable">Email</th>
        <th>Actions</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td>John Doe</td>
        <td>john@example.com</td>
        <td>
          <div class="table-actions">
            <button class="table-action-btn">Edit</button>
          </div>
        </td>
      </tr>
    </tbody>
  </table>
  <div class="table-pagination">
    <div class="pagination-info">Showing 1-10 of 100</div>
    <div class="pagination-controls">
      <button class="pagination-btn">Previous</button>
      <button class="pagination-btn active">1</button>
      <button class="pagination-btn">Next</button>
    </div>
  </div>
</div>
```

**Table Variants**:
- `table-striped`: Alternating row colors
- `table-bordered`: Borders around cells
- `table-compact`: Reduced padding
- `table-fixed`: Fixed table layout

### Cards

```html
<div class="card">
  <div class="card-header">
    <h2 class="card-title">Title</h2>
    <p class="card-description">Description text</p>
  </div>
  <div class="card-content">
    <!-- Content -->
  </div>
  <div class="card-footer">
    <button class="btn btn-primary">Action</button>
  </div>
</div>
```

### Alerts

```html
<div class="alert alert-info">
  <div>
    <div class="alert-title">Info</div>
    <div class="alert-description">Message text</div>
  </div>
</div>

<!-- Variants: alert-info, alert-success, alert-warning, alert-error -->
```

### Badges

```html
<span class="badge badge-primary">Primary</span>
<span class="badge badge-success">Success</span>
<span class="badge badge-warning">Warning</span>
<span class="badge badge-error">Error</span>
```

## Layout Utilities

### Flexbox

```html
<div class="flex items-center justify-between gap-4">
  <div class="flex-1">Content</div>
  <div>Sidebar</div>
</div>
```

### Spacing

```html
<div class="p-4 m-2 px-6 py-3"></div>
<div class="mt-8 mb-4 ml-2 mr-2"></div>
```

### Text Utilities

```html
<p class="text-center text-lg font-semibold">Text</p>
<p class="truncate">Long text that will be truncated...</p>
```

## Theme Switching

The design system automatically detects system preferences but also supports manual theme switching:

```typescript
// Set theme
document.documentElement.setAttribute('data-theme', 'dark');

// Get current theme
const theme = document.documentElement.getAttribute('data-theme');

// Store preference
localStorage.setItem('theme', 'dark');
```

## File Structure

```
src/renderer/
├── design-system/
│   ├── colors.scss       # Color tokens and theme variables
│   ├── typography.scss   # Font families, sizes, and text styles
│   ├── spacing.scss      # Spacing scale and layout utilities
│   ├── button.scss       # Button component styles
│   ├── input.scss        # Input and form control styles
│   ├── dropdown.scss     # Dropdown and select styles
│   └── table.scss        # Table component styles
└── styles/
    └── main.scss         # Main stylesheet (imports all components)
```

## Usage

Import the main stylesheet in your renderer process:

```typescript
import './styles/main.scss';
```

All design system styles will be automatically loaded and available.

## Browser Support

- Chromium 120+ (Electron 33+)
- Modern CSS features: CSS Custom Properties, Flexbox, Grid

## Customization

To customize the design system, modify the SCSS variables in the individual component files:

```scss
// colors.scss
$light-accent-primary: #0078d4; // Change primary accent color

// typography.scss
$font-size-base: 1rem; // Change base font size

// spacing.scss
$spacing-4: 1rem; // Change spacing unit
```

## Best Practices

1. **Use semantic HTML**: Prefer `<button>` over `<div>` for clickable elements
2. **Maintain contrast ratios**: Ensure text meets WCAG AA standards
3. **Test both themes**: Always verify components in light and dark modes
4. **Use utility classes sparingly**: Prefer component classes for maintainability
5. **Follow naming conventions**: Use BEM-like naming for custom components
6. **Respect system preferences**: Don't force a theme unless explicitly requested

## Accessibility

- All interactive elements are keyboard accessible
- Focus states are clearly visible
- Color contrast ratios meet WCAG AA standards
- Semantic HTML is used throughout
- ARIA attributes are applied where necessary

## Performance

- CSS is bundled and minified in production
- No runtime JavaScript for styling (except theme toggle)
- Efficient CSS selectors
- Minimal specificity conflicts

## Contributing

When adding new components:

1. Create a new SCSS file in `design-system/`
2. Import it in `styles/main.scss`
3. Follow existing naming conventions
4. Document usage in this file
5. Test in both themes
6. Ensure accessibility compliance

## License

MIT License - See LICENSE file for details
