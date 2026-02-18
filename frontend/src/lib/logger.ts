import pino from 'pino';

const level = import.meta.env.DEV ? 'debug' : 'info';

export const logger = pino({
  browser: {
    asObject: true,
  },
  level,
});
