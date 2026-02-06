# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with the documentation in this repository.

## Project Overview

**Candy Documentation** is the official documentation for the Candy web server project. It is built using Docusaurus 3.x, a modern static website generator for technical documentation.

Key features:

- Multi-language support (Chinese and English)
- Blog system for news and updates
- Versioned documentation
- Search functionality
- Responsive design
- Dark mode support

## Build, Lint, and Test Commands

### Installation

```bash
# Install dependencies
npm install
# or
pnpm install
```

### Development

```bash
# Start development server (with hot reload)
npm start
# or
pnpm start

# Start development server on specific port
npm start -- --port 3001
# or
pnpm start -- --port 3001
```

### Build

```bash
# Build for production
npm run build
# or
pnpm build

# Build and serve locally
npm run serve
# or
pnpm serve
```

### Linting and Formatting

```bash
# Run ESLint to check for errors
npm run lint
# or
pnpm lint

# Auto-fix lint issues
npm run lint:fix
# or
pnpm lint:fix

# Format code with Prettier
npm run format
# or
pnpm format

# Check for format issues
npm run format:check
# or
pnpm format:check
```

### Translation

```bash
# Extract translation messages
npm run write-translations
# or
pnpm write-translations

# Build for specific locale
npm run build -- --locale zh-CN
# or
pnpm build -- --locale zh-CN
```

## Project Structure

```
docs/
├── docusaurus.config.ts    # Docusaurus configuration
├── sidebars.ts            # Sidebar configuration for documentation
├── tsconfig.json          # TypeScript configuration
├── package.json           # Dependencies and scripts
├── pnpm-lock.yaml         # PNPM lock file
├── README.md              # Project README
├── Makefile               # Makefile for common tasks
├── src/                   # Source code
│   ├── components/        # Reusable React components
│   ├── css/               # Global styles
│   ├── pages/             # Custom pages (not part of documentation)
│   └── theme/             # Theme customizations
├── docs/                  # Documentation files (Markdown/MDX)
│   ├── intro.md           # Introduction page
│   └── ...                # Other documentation files
├── blog/                  # Blog posts (Markdown/MDX)
│   ├── 2024-01-01-post.md # Blog post
│   └── ...
├── i18n/                  # Internationalization files
│   ├── zh-CN/             # Chinese translations
│   └── ...
├── static/                # Static assets
│   ├── img/               # Images
│   └── ...
└── build/                 # Build output directory
```

## Key Files and Directories

### Configuration Files

- `docusaurus.config.ts`: Main Docusaurus configuration file. Contains site metadata, theme config, plugin config, and more.
- `sidebars.ts`: Defines the structure of the documentation sidebar.
- `tsconfig.json`: TypeScript configuration for the project.

### Documentation Content

- `docs/`: Contains all documentation files in Markdown or MDX format. Files in this directory are automatically included in the documentation.
- `blog/`: Contains blog posts in Markdown or MDX format. Posts are automatically sorted by date.
- `src/pages/`: Contains custom React pages that are not part of the documentation. These pages are accessible directly from the site's navigation.

### Theme and Styling

- `src/components/`: Reusable React components that can be used in documentation or pages.
- `src/css/`: Global styles for the site. The main file is `custom.css`.
- `src/theme/`: Theme customizations. Overrides default Docusaurus theme components.

### Static Assets

- `static/`: Contains static assets such as images, PDFs, and other files. These assets can be referenced in documentation using `/static/` prefix.

## Writing Documentation

### Markdown Features

Docusaurus supports standard Markdown syntax with additional features:

- **MDX**: Allows embedding React components in Markdown files
- **Code blocks**: Syntax highlighting for various languages
- **Admonitions**: Note, tip, info, caution, danger boxes
- **Tables**: Markdown tables with support for sorting
- **Links**: Auto-generated links to other documentation pages
- **Images**: Support for responsive images

### Example: Admonition

```markdown
:::note

This is a note admonition.

:::

:::tip

This is a tip admonition.

:::

:::info

This is an info admonition.

:::

:::caution

This is a caution admonition.

:::

:::danger

This is a danger admonition.

:::
```

### Example: Code Block

```javascript
// This is a JavaScript code block
function hello() {
  console.log('Hello, World!');
}
```

## Adding a New Documentation Page

1. Create a new Markdown/MDX file in the `docs/` directory
2. Add frontmatter to the file:
   ```markdown
   ---
   title: Page Title
   description: Page description
   sidebar_label: Sidebar Label
   ---
   ```
3. Add content using Markdown/MDX syntax
4. Update `sidebars.ts` to include the new page in the sidebar

## Adding a New Blog Post

1. Create a new Markdown/MDX file in the `blog/` directory
2. File name should follow the format: `YYYY-MM-DD-post-slug.md`
3. Add frontmatter to the file:
   ```markdown
   ---
   title: Blog Post Title
   description: Blog post description
   authors:
     - name: Author Name
       title: Author Title
       url: Author URL
       image_url: Author Image URL
   tags: [tag1, tag2]
   ---
   ```
4. Add content using Markdown/MDX syntax

## Theme Customization

### Overriding Theme Components

1. Create a new file in `src/theme/` directory with the same name as the component you want to override
2. Export a React component that replaces the default
3. Docusaurus will automatically use your custom component

### Customizing Styles

1. Modify `src/css/custom.css` to add global styles
2. Use CSS variables defined in `src/css/variables.css` for consistent styling
3. For component-specific styles, use CSS modules

## Deployment

### Local Build

```bash
# Build for production
npm run build
# or
pnpm build

# Serve locally
npm run serve
# or
pnpm serve
```

### Deployment to GitHub Pages

```bash
# Deploy to GitHub Pages
npm run deploy
# or
pnpm deploy
```

### Other Deployment Options

- Netlify
- Vercel
- AWS S3
- Any static hosting service

## Common Tasks

### Updating Dependencies

```bash
# Update all dependencies
npm update
# or
pnpm update

# Update specific package
npm install package@latest
# or
pnpm add package@latest
```

### Clearing Cache

```bash
# Clear Docusaurus cache
rm -rf .docusaurus

# Clear node_modules and reinstall
rm -rf node_modules package-lock.json
npm install
```

### Previewing Changes

```bash
# Start development server
npm start
# or
pnpm start
```

Open http://localhost:3000 in your browser to preview changes.

## Troubleshooting

### Common Issues

1. **Build fails**: Check for syntax errors in Markdown/MDX files, missing dependencies, or configuration issues.
2. **Development server won't start**: Clear cache, reinstall dependencies, or check for port conflicts.
3. **Changes not reflected**: Clear browser cache, restart development server, or check if file is properly saved.
4. **Translation issues**: Make sure translation files are up to date and follow the correct format.

### Debugging Tips

- Check console output for errors
- Use browser DevTools to inspect elements and debug styles
- Look at Docusaurus logs for detailed error messages

## Resources

- [Docusaurus Documentation](https://docusaurus.io/docs)
- [Markdown Guide](https://www.markdownguide.org/)
- [MDX Documentation](https://mdxjs.com/docs/)
- [React Documentation](https://react.dev/)
