# React Components Guide

This document provides a guide to using the React components in the QueryBox design system.

## Overview

The design system has been converted to React components while maintaining the existing SCSS styling. All components are fully typed with TypeScript and follow React best practices.

## Getting Started

### Basic Usage

```tsx
import { Button, Input, Card } from './components';

function MyComponent() {
  return (
    <Card>
      <Input label="Name" placeholder="Enter your name" />
      <Button variant="primary">Submit</Button>
    </Card>
  );
}
```

### Theme Management

The application includes a theme context for managing light/dark mode:

```tsx
import { useTheme } from './hooks/useTheme';

function ThemeToggle() {
  const { theme, toggleTheme } = useTheme();
  
  return (
    <Button onClick={toggleTheme}>
      {theme === 'light' ? '🌙' : '☀️'}
    </Button>
  );
}
```

## Component Library

### Button

The Button component supports multiple variants and sizes.

```tsx
<Button variant="primary" size="lg">Click me</Button>
<Button variant="secondary" disabled>Disabled</Button>
<Button variant="danger" onClick={handleDelete}>Delete</Button>
```

**Props:**
- `variant`: 'primary' | 'secondary' | 'outline' | 'ghost' | 'danger' | 'success'
- `size`: 'sm' | 'md' | 'lg'
- `fullWidth`: boolean
- `icon`: boolean
- All standard HTML button attributes

### Input

Text input component with label, hint, and error message support.

```tsx
<Input
  label="Username"
  placeholder="Enter username"
  required
  hint="Choose a unique username"
  error={errors.username}
/>
```

**Props:**
- `label`: string
- `hint`: string
- `error`: string
- `required`: boolean
- `inputSize`: 'sm' | 'md' | 'lg'
- All standard HTML input attributes

### TextArea

Multi-line text input component.

```tsx
<TextArea
  label="Description"
  placeholder="Enter description"
  rows={5}
/>
```

### Checkbox

Checkbox component with optional label.

```tsx
<Checkbox label="Accept terms" defaultChecked />
<Checkbox label="Subscribe" />
```

### Radio

Radio button component with optional label.

```tsx
<Radio name="option" label="Option 1" defaultChecked />
<Radio name="option" label="Option 2" />
```

### Select

Select dropdown component.

```tsx
<Select
  label="Choose option"
  options={[
    { value: '1', label: 'Option 1' },
    { value: '2', label: 'Option 2' }
  ]}
/>
```

### Dropdown

Custom dropdown menu component with full control over content.

```tsx
<Dropdown trigger={<Button>Menu</Button>}>
  <DropdownHeader>Actions</DropdownHeader>
  <DropdownItem onClick={handleEdit}>Edit</DropdownItem>
  <DropdownDivider />
  <DropdownItem onClick={handleDelete}>Delete</DropdownItem>
</Dropdown>
```

### Card

Card container component with header, content, and footer sections.

```tsx
<Card>
  <CardHeader title="Card Title" description="Optional description" />
  <CardContent>
    <p>Card content goes here</p>
  </CardContent>
  <CardFooter>
    <Button variant="primary">Action</Button>
  </CardFooter>
</Card>
```

### Alert

Alert/notification component with different variants.

```tsx
<Alert 
  variant="info" 
  title="Info" 
  description="This is an informational message" 
/>
<Alert variant="success" title="Success" />
<Alert variant="warning" title="Warning" />
<Alert variant="error" title="Error" />
```

### Badge

Badge component for status indicators.

```tsx
<Badge variant="primary">New</Badge>
<Badge variant="success">Active</Badge>
<Badge variant="error">Inactive</Badge>
```

### Table

Table component with sorting and pagination support.

```tsx
<Table striped>
  <TableHead>
    <TableRow>
      <TableHeaderCell sortable>Name</TableHeaderCell>
      <TableHeaderCell sortable>Email</TableHeaderCell>
      <TableHeaderCell>Actions</TableHeaderCell>
    </TableRow>
  </TableHead>
  <TableBody>
    <TableRow>
      <TableCell>John Doe</TableCell>
      <TableCell>john@example.com</TableCell>
      <TableCell>
        <Button size="sm">Edit</Button>
      </TableCell>
    </TableRow>
  </TableBody>
</Table>
<TablePagination
  currentPage={1}
  totalPages={10}
  onPageChange={setPage}
  totalItems={100}
  itemsPerPage={10}
/>
```

**Table Props:**
- `striped`: boolean - Alternating row colors
- `bordered`: boolean - Borders around cells
- `compact`: boolean - Reduced padding
- `fixed`: boolean - Fixed table layout

## Component Architecture

### File Structure

```
src/renderer/
├── components/
│   ├── ui/               # UI components
│   │   ├── Button.tsx
│   │   ├── Input.tsx
│   │   ├── Select.tsx
│   │   ├── Alert.tsx
│   │   ├── Badge.tsx
│   │   └── Table.tsx
│   ├── layout/           # Layout components
│   │   └── Card.tsx
│   └── index.ts          # Central export
├── hooks/
│   └── useTheme.tsx      # Theme management hook
├── design-system/        # SCSS files (unchanged)
├── styles/
│   └── main.scss         # Main stylesheet
├── App.tsx               # Main application component
└── renderer.tsx          # React entry point
```

### Design Principles

1. **Minimal Changes**: Components wrap existing SCSS styles without modifying them
2. **Type Safety**: All components are fully typed with TypeScript
3. **Composition**: Components can be easily composed together
4. **Accessibility**: Components maintain keyboard navigation and ARIA attributes
5. **Consistency**: All components follow the same naming and pattern conventions

## Styling

All styling is handled through the existing SCSS design system. Components apply appropriate CSS classes based on props. The SCSS files remain unchanged and continue to provide all visual styling.

## Theme Support

The theme system works through CSS custom properties defined in the SCSS files. The React `ThemeProvider` manages the `data-theme` attribute on the root element, which automatically switches between light and dark mode.

## Migration Guide

If you're migrating from the vanilla JavaScript version:

1. Replace HTML elements with React components
2. Convert event handlers from DOM to React (e.g., `onClick` instead of `addEventListener`)
3. Use React state management instead of direct DOM manipulation
4. Import components from the central export: `import { Button } from './components'`

## Future Enhancements

Potential improvements that could be made:

- Add form validation with React Hook Form
- Add animation with Framer Motion
- Add data fetching with React Query
- Add more complex components (Modal, Toast, Tabs, etc.)
- Add unit tests with Jest and React Testing Library
- Add Storybook for component documentation

## License

MIT
