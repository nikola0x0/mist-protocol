/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: 'class', // Use class-based dark mode for better control
  theme: {
    extend: {
      fontFamily: {
        sans: ['Unbounded', 'system-ui', 'sans-serif'],
      },
      colors: {
        // Sui-inspired primary colors
        sui: {
          50: '#e6f4ff',
          100: '#b3dfff',
          200: '#80caff',
          300: '#4db5ff',
          400: '#1aa0ff',
          500: '#4DA2FF', // Sui Blue
          600: '#0066FF', // Electric Blue
          700: '#0052cc',
          800: '#003d99',
          900: '#002966',
        },
        // Web3 accent colors
        cyber: {
          cyan: '#00D1FF',
          red: '#F87171',      // Light red
          pink: '#FDA4AF',     // Light pink / rose
          green: '#00FF88',
        },
        // Dark backgrounds
        dark: {
          900: '#030712', // Near black
          800: '#0A0E27', // Deep navy
          700: '#111827',
          600: '#1F2937',
          500: '#374151',
        },
      },
      backgroundImage: {
        // Gradients for web3 feel
        'gradient-radial': 'radial-gradient(ellipse at center, var(--tw-gradient-stops))',
        'gradient-conic': 'conic-gradient(from 180deg at 50% 50%, var(--tw-gradient-stops))',
        'sui-gradient': 'linear-gradient(135deg, #4DA2FF 0%, #00D1FF 100%)',
        'cyber-gradient': 'linear-gradient(135deg, #0066FF 0%, #4DA2FF 50%, #00D1FF 100%)',
        'dark-gradient': 'linear-gradient(180deg, #0A0E27 0%, #030712 100%)',
        'glass-gradient': 'linear-gradient(135deg, rgba(255,255,255,0.1) 0%, rgba(255,255,255,0.05) 100%)',
      },
      boxShadow: {
        'glow-sm': '0 0 15px rgba(77, 162, 255, 0.3)',
        'glow-md': '0 0 30px rgba(77, 162, 255, 0.4)',
        'glow-lg': '0 0 50px rgba(77, 162, 255, 0.5)',
        'glow-cyan': '0 0 30px rgba(0, 209, 255, 0.4)',
        'glow-pink': '0 0 30px rgba(253, 164, 175, 0.4)',
        'inner-glow': 'inset 0 0 20px rgba(77, 162, 255, 0.1)',
      },
      backdropBlur: {
        xs: '2px',
      },
      animation: {
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'glow': 'glow 2s ease-in-out infinite alternate',
        'float': 'float 6s ease-in-out infinite',
      },
      keyframes: {
        glow: {
          '0%': { boxShadow: '0 0 20px rgba(77, 162, 255, 0.3)' },
          '100%': { boxShadow: '0 0 40px rgba(77, 162, 255, 0.6)' },
        },
        float: {
          '0%, 100%': { transform: 'translateY(0px)' },
          '50%': { transform: 'translateY(-10px)' },
        },
      },
    },
  },
  plugins: [],
}
