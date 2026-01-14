/** @type {import('tailwindcss').Config} */
module.exports = {
  content: {
    files: ["*.html", "./src/**/*.rs"],
  },
  theme: {
    extend: {},
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
