import { describe, expect, it } from 'vitest';
import { createEntityStore } from './entityStore';

interface TestEntity {
  id: string;
  name: string;
}

describe('createEntityStore', () => {
  it('starts with empty state', () => {
    const useStore = createEntityStore<TestEntity>();
    const state = useStore.getState();
    expect(state.ids).toEqual([]);
    expect(state.entities).toEqual({});
  });

  it('setAll replaces all entities', () => {
    const useStore = createEntityStore<TestEntity>();
    useStore.getState().setAll([
      { id: '1', name: 'A' },
      { id: '2', name: 'B' },
    ]);
    const state = useStore.getState();
    expect(state.ids).toEqual(['1', '2']);
    expect(state.entities['1']).toEqual({ id: '1', name: 'A' });
    expect(state.entities['2']).toEqual({ id: '2', name: 'B' });
  });

  it('setOne inserts a new entity', () => {
    const useStore = createEntityStore<TestEntity>();
    useStore.getState().setOne({ id: '1', name: 'A' });
    const state = useStore.getState();
    expect(state.ids).toEqual(['1']);
    expect(state.entities['1']).toEqual({ id: '1', name: 'A' });
  });

  it('setOne updates an existing entity', () => {
    const useStore = createEntityStore<TestEntity>();
    useStore.getState().setOne({ id: '1', name: 'A' });
    useStore.getState().setOne({ id: '1', name: 'Updated' });
    const state = useStore.getState();
    expect(state.ids).toEqual(['1']);
    expect(state.entities['1']).toEqual({ id: '1', name: 'Updated' });
  });

  it('removeOne removes entity by id', () => {
    const useStore = createEntityStore<TestEntity>();
    useStore.getState().setAll([
      { id: '1', name: 'A' },
      { id: '2', name: 'B' },
    ]);
    useStore.getState().removeOne('1');
    const state = useStore.getState();
    expect(state.ids).toEqual(['2']);
    expect(state.entities['1']).toBeUndefined();
  });

  it('removeOne is no-op for non-existent id', () => {
    const useStore = createEntityStore<TestEntity>();
    useStore.getState().setOne({ id: '1', name: 'A' });
    useStore.getState().removeOne('999');
    expect(useStore.getState().ids).toEqual(['1']);
  });

  it('getById returns entity or undefined', () => {
    const useStore = createEntityStore<TestEntity>();
    useStore.getState().setOne({ id: '1', name: 'A' });
    expect(useStore.getState().getById('1')).toEqual({ id: '1', name: 'A' });
    expect(useStore.getState().getById('999')).toBeUndefined();
  });

  it('getAll returns entities in id order', () => {
    const useStore = createEntityStore<TestEntity>();
    useStore.getState().setAll([
      { id: '2', name: 'B' },
      { id: '1', name: 'A' },
    ]);
    const all = useStore.getState().getAll();
    expect(all).toEqual([
      { id: '2', name: 'B' },
      { id: '1', name: 'A' },
    ]);
  });

  it('clear removes all entities', () => {
    const useStore = createEntityStore<TestEntity>();
    useStore.getState().setAll([
      { id: '1', name: 'A' },
      { id: '2', name: 'B' },
    ]);
    useStore.getState().clear();
    expect(useStore.getState().ids).toEqual([]);
    expect(useStore.getState().entities).toEqual({});
  });
});
