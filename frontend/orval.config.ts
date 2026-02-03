import { defineConfig } from 'orval';

export default defineConfig({
  gantryBoard: {
    input: {
      target: '../openapi.json',
    },
    output: {
      target: './src/api/generated/endpoints',
      schemas: './src/api/generated/model',
      client: 'react-query',
      mode: 'tags-split',
      override: {
        mutator: {
          path: './src/api/client.ts',
          name: 'customInstance',
        },
      },
    },
  },
});
