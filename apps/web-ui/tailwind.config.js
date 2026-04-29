/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        "bg-page": "var(--bg-page)",
        "bg-surface": "var(--bg-surface)",
        "bg-surface-2": "var(--bg-surface-2)",
        fg: "var(--fg)",
        "fg-muted": "var(--fg-muted)",
        "fg-subtle": "var(--fg-subtle)",
        border: "var(--border)",
        "border-soft": "var(--border-soft)",
        "border-strong": "var(--border-strong)",
        "status-done": "var(--status-done)",
        "status-active": "var(--status-active)",
        "status-queued": "var(--status-queued)",
        "status-failed": "var(--status-failed)",
        "status-info": "var(--status-info)",
        "agent-architect": "var(--agent-architect)",
        "agent-developer": "var(--agent-developer)",
        "agent-qa": "var(--agent-qa)",
        "agent-reviewer": "var(--agent-reviewer)",
      },
      fontFamily: {
        sans: ["Inter", "ui-sans-serif", "system-ui", "-apple-system", "Segoe UI", "Roboto", "sans-serif"],
        mono: ["ui-monospace", "SFMono-Regular", "Menlo", "Monaco", "Consolas", "Liberation Mono", "monospace"],
      },
      boxShadow: {
        card: "var(--shadow-xs)",
        popover: "var(--shadow-md)",
        modal: "var(--shadow-lg)",
      },
    },
  },
  plugins: [],
};
