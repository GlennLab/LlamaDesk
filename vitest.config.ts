import react from '@vitejs/plugin-react'
import { defineConfig } from 'vitest/config'

// Test-only config, kept separate from vite.config.ts so the dev/build server
// settings stay untouched. Vitest picks this file up automatically.
export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'text-summary'],
      include: ['src/**/*.{ts,tsx}'],
      exclude: [
        'src/**/*.test.{ts,tsx}',
        'src/**/*.d.ts',
        'src/main.tsx',
        'src/test/**',
      ],
    },
  },
})
