/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{ts,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // All colors are driven by CSS custom properties so themes can override them at runtime.
        bg: {
          900: "var(--color-bg-900)",
          800: "var(--color-bg-800)",
          700: "var(--color-bg-700)",
          600: "var(--color-bg-600)",
          500: "var(--color-bg-500)",
        },
        accent: {
          500: "var(--color-accent-500)",
          600: "var(--color-accent-600)",
          400: "var(--color-accent-400)",
        },
        surface: {
          900: "var(--color-surface-900)",
          800: "var(--color-surface-800)",
          700: "var(--color-surface-700)",
        },
        muted: "var(--color-muted)",
        online: "var(--color-online)",
        idle: "var(--color-idle)",
        dnd: "var(--color-dnd)",
        offline: "var(--color-offline)",
        fg: "var(--color-fg)",
      },
      fontFamily: {
        sans: ["Inter", "system-ui", "sans-serif"],
        mono: ["JetBrains Mono", "Fira Code", "monospace"],
      },
    },
  },
  plugins: [],
};
