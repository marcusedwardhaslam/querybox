// Renderer process
import './styles/main.scss';

// Theme toggle functionality
function initThemeToggle() {
  const themeToggle = document.getElementById('theme-toggle');
  const themeIcon = document.getElementById('theme-icon');
  const root = document.documentElement;

  // Check for saved theme preference or default to system preference
  const savedTheme = localStorage.getItem('theme');
  const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
  
  let currentTheme = savedTheme || (systemPrefersDark ? 'dark' : 'light');
  applyTheme(currentTheme);

  themeToggle?.addEventListener('click', () => {
    currentTheme = currentTheme === 'light' ? 'dark' : 'light';
    applyTheme(currentTheme);
    localStorage.setItem('theme', currentTheme);
  });

  function applyTheme(theme: string) {
    root.setAttribute('data-theme', theme);
    if (themeIcon) {
      themeIcon.textContent = theme === 'light' ? '🌙' : '☀️';
    }
  }
}

// Dropdown functionality
function initDropdowns() {
  const dropdownTrigger = document.getElementById('dropdown-trigger');
  const dropdownMenu = document.getElementById('dropdown-menu');

  if (dropdownTrigger && dropdownMenu) {
    dropdownTrigger.addEventListener('click', (e) => {
      e.stopPropagation();
      dropdownMenu.classList.toggle('dropdown-open');
    });

    // Close dropdown when clicking outside
    document.addEventListener('click', () => {
      dropdownMenu.classList.remove('dropdown-open');
    });

    // Prevent closing when clicking inside the dropdown
    dropdownMenu.addEventListener('click', (e) => {
      e.stopPropagation();
    });
  }
}

// Smooth scroll for navigation
function initSmoothScroll() {
  const navLinks = document.querySelectorAll('.nav-link');
  
  navLinks.forEach(link => {
    link.addEventListener('click', (e) => {
      e.preventDefault();
      const href = link.getAttribute('href');
      if (href) {
        const target = document.querySelector(href);
        if (target) {
          target.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }
      }
    });
  });
}

// Table sorting demo
function initTableSorting() {
  const sortableHeaders = document.querySelectorAll('.sortable');
  
  sortableHeaders.forEach(header => {
    header.addEventListener('click', () => {
      // Remove sorted classes from other headers
      sortableHeaders.forEach(h => {
        if (h !== header) {
          h.classList.remove('sorted-asc', 'sorted-desc');
        }
      });

      // Toggle sorting
      if (header.classList.contains('sorted-asc')) {
        header.classList.remove('sorted-asc');
        header.classList.add('sorted-desc');
      } else if (header.classList.contains('sorted-desc')) {
        header.classList.remove('sorted-desc');
      } else {
        header.classList.add('sorted-asc');
      }
    });
  });
}

// Initialize all components when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
  initThemeToggle();
  initDropdowns();
  initSmoothScroll();
  initTableSorting();
  
  console.log('QueryBox Design System initialized');
});

// Add sidebar nav link styling
const style = document.createElement('style');
style.textContent = `
  .nav-link {
    display: block;
    padding: var(--spacing-2) var(--spacing-4);
    color: var(--text-secondary);
    text-decoration: none;
    border-radius: var(--radius-md);
    transition: all 0.15s ease-in-out;
    font-size: var(--font-size-sm);
    margin-bottom: var(--spacing-1);
  }

  .nav-link:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  .nav-link:active {
    background-color: var(--bg-active);
  }
`;
document.head.appendChild(style);
