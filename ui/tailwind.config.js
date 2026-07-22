/** Design tokens from docs/design/simorgh-ui.html — the visual source of truth. */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        canvas: "#191919",
        surface: "#202020",
        raised: "#383836",
        accent: "#5E9FE8",
        positive: "#72BC8F",
        attention: "#DE9255",
        danger: "#E97366",
        // Light theme (wizard)
        lcanvas: "#FFFFFF",
        lsoft: "#F9F8F7",
        lsurface: "#F0EFED",
        lborder: "#E6E5E3",
        ltext: "#2C2C2B",
        lmuted: "#7D7A75",
        laccent: "#2783DE",
        lpositive: "#46A171",
      },
      borderColor: { DEFAULT: "rgba(255,255,255,0.20)" },
      textColor: { secondary: "rgba(255,255,255,0.65)" },
      borderRadius: { DEFAULT: "8px", card: "12px" },
      fontFamily: {
        sans: ["-apple-system", "BlinkMacSystemFont", "Segoe UI", "Roboto", "Helvetica", "Arial", "sans-serif"],
        fa: ["Vazirmatn", "Noto Sans Arabic", "-apple-system", "Segoe UI", "sans-serif"],
      },
      spacing: { 1.5: "6px" },
    },
  },
  plugins: [],
}
