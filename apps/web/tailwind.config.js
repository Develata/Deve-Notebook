/** @type {import('tailwindcss').Config} */
module.exports = {
  content: {
    files: ["*.html", "./src/**/*.rs"],
  },
  theme: {
    extend: {
      backgroundColor: {
        app: 'var(--bg-app)',
        sidebar: 'var(--bg-sidebar)',
        editor: 'var(--bg-editor)',
        panel: 'var(--bg-panel)',
        'status': 'var(--bg-statusbar)',
        'status-ro': 'var(--bg-statusbar-ro)',
        input: 'var(--bg-input)',
        hover: 'var(--bg-hover)',
        active: 'var(--bg-active)',
        accent: 'var(--bg-accent)',
        'accent-hover': 'var(--bg-accent-hover)',
        'accent-subtle': 'var(--bg-accent-subtle)',
        overlay: 'var(--bg-overlay)',
        tooltip: 'var(--bg-tooltip)',
        badge: 'var(--bg-badge)',
        'badge-count': 'var(--bg-badge-count)',
        'chat-user': 'var(--bg-chat-user)',
        'badge-success': 'var(--bg-badge-success)',
      },
      textColor: {
        primary: 'var(--fg-primary)',
        secondary: 'var(--fg-secondary)',
        muted: 'var(--fg-muted)',
        accent: 'var(--fg-accent)',
        'on-accent': 'var(--fg-on-accent)',
        'on-tooltip': 'var(--fg-on-tooltip)',
        added: 'var(--color-added)',
        modified: 'var(--color-modified)',
        deleted: 'var(--color-deleted)',
        warning: 'var(--color-warning)',
        info: 'var(--color-info)',
        'badge-success': 'var(--fg-badge-success)',
      },
      borderColor: {
        default: 'var(--border-default)',
        'b-hover': 'var(--border-hover)',
        'b-active': 'var(--border-active)',
        'b-accent': 'var(--border-accent)',
        'badge-success': 'var(--border-badge-success)',
      },
      ringColor: {
        accent: 'var(--border-accent)',
      },
      accentColor: {
        accent: 'var(--bg-accent)',
      },
    },
  },
  plugins: [],
  safelist: [
    "group-hover:flex",
    "group-hover:!flex",
    "group-hover:inline-block",
    "group-hover:block",
    "w-3.5",
    "h-3.5"
  ],
};
