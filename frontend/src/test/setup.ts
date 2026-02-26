import '@testing-library/jest-dom/vitest';
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import { afterAll, afterEach, beforeAll } from 'vitest';
import en from '../locales/en.json';

// Polyfills for Radix UI (Dialog, Select, etc.) in jsdom
globalThis.ResizeObserver ??= class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
} as unknown as typeof ResizeObserver;

HTMLElement.prototype.scrollIntoView ??= () => {};
HTMLElement.prototype.hasPointerCapture ??= () => false;

i18n.use(initReactI18next).init({
  resources: { en: { translation: en } },
  lng: 'en',
  fallbackLng: 'en',
  interpolation: { escapeValue: false },
});

import { useAgentStore } from '../stores/agentStore';
import { useAuthStore } from '../stores/authStore';
import { useBoardStore } from '../stores/boardStore';
import { useToastStore } from '../stores/toastStore';
import { useUiStore } from '../stores/uiStore';
import { server } from './mocks/server';

beforeAll(() => server.listen({ onUnhandledRequest: 'warn' }));

afterEach(() => {
  server.resetHandlers();
  useAgentStore.setState(useAgentStore.getInitialState());
  useAuthStore.setState(useAuthStore.getInitialState());
  useBoardStore.setState(useBoardStore.getInitialState());
  useToastStore.setState(useToastStore.getInitialState());
  useUiStore.setState(useUiStore.getInitialState());
  sessionStorage.clear();
});

afterAll(() => server.close());
