/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{ts,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Discord-inspired dark palette, but distinctly "Nexus"
        bg: {
          900: "#0d0f13",
          800: "#161a22",
          700: "#1e2330",
          600: "#262d3d",
          500: "#2f3850",
        },
        accent: {
          500: "#7c6af7",
          600: "#6b59e8",
          400: "#9d8fff",
        },
        surface: {
          900: "#111318",
          800: "#191d26",
          700: "#222736",
        },
        muted: "#8892a4",
        online: "#3ba55c",
        idle: "#faa81a",
        dnd: "#ed4245",
        offline: "#747f8d",
      },
      fontFamily: {
        sans: ["Inter", "system-ui", "sans-serif"],
        mono: ["JetBrains Mono", "Fira Code", "monospace"],
      },
    },
  },
  plugins: [],
};
