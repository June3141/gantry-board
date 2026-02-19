import '@testing-library/jest-dom/vitest';
import { afterEach } from 'vitest';

import { useAgentStore } from '../stores/agentStore';
import { useAuthStore } from '../stores/authStore';
import { useBoardStore } from '../stores/boardStore';
import { useToastStore } from '../stores/toastStore';
import { useUiStore } from '../stores/uiStore';

afterEach(() => {
  useAgentStore.setState(useAgentStore.getInitialState());
  useAuthStore.setState(useAuthStore.getInitialState());
  useBoardStore.setState(useBoardStore.getInitialState());
  useToastStore.setState(useToastStore.getInitialState());
  useUiStore.setState(useUiStore.getInitialState());
  sessionStorage.clear();
});
