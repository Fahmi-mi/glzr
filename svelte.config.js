import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const config = {
  preprocess: vitePreprocess(),
  compilerOptions: {
    warningFilter: (warning) =>
      !warning.code.startsWith('a11y_') && warning.code !== 'state_referenced_locally'
  },
  kit: {
    adapter: adapter({
      fallback: 'index.html'
    }),
    paths: {
      relative: true
    }
  }
};

export default config;
