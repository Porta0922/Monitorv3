/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        primary: '#0066cc',
        success: '#00aa00',
        danger: '#cc3333',
        warning: '#ffaa00',
      },
    },
  },
  plugins: [],
}
