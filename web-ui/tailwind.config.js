/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // 终端风格配色
        'terminal': {
          bg: '#0a0a0f',
          surface: '#12121a',
          border: '#1e1e2e',
          text: '#cdd6f4',
          muted: '#6c7086',
          accent: '#00d9ff',
          success: '#a6e3a1',
          warning: '#f9e2af',
          error: '#f38ba8',
        },
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'Fira Code', 'Consolas', 'monospace'],
      },
    },
  },
  plugins: [],
}
