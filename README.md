# QueryBox

A free and open source SQL GUI made in Electron with TypeScript.

## Features

- 🎨 **Native-Looking Design System** - Beautiful UI that feels at home on any platform
- 🌓 **Dark/Light Mode** - Automatic theme switching based on system preferences
- 💻 **TypeScript** - Full type safety and modern development experience
- ⚡ **Electron** - Cross-platform desktop application

## Getting Started

### Prerequisites

- Node.js 18+ and npm

### Installation

```bash
npm install
```

### Development

```bash
# Build the application
npm run build:dev

# Start the application
npm start

# Watch mode (auto-rebuild on changes)
npm run dev
```

### Production Build

```bash
# Create optimized production build
npm run build

# Package the application for distribution
npm run package
```

## Design System

QueryBox includes a comprehensive design system with:

- **Color Schemes**: Light and dark mode with native-inspired colors
- **Typography**: System fonts for native appearance
- **Components**: Buttons, inputs, dropdowns, tables, and more
- **Layout Utilities**: Flexbox, spacing, and responsive utilities

For detailed documentation, see [DESIGN_SYSTEM.md](DESIGN_SYSTEM.md).

## Project Structure

```
querybox/
├── src/
│   ├── main/              # Electron main process
│   │   └── main.ts
│   └── renderer/          # Electron renderer process
│       ├── components/
│       ├── design-system/ # Design system SCSS files
│       ├── styles/        # Main stylesheet
│       ├── index.html
│       └── renderer.ts
├── dist/                  # Build output
├── tsconfig.json
├── webpack.config.js
└── package.json
```

## License

MIT
