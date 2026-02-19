import { useState } from 'react';
import type { ProjectMember } from '@/api/generated/model';
import { TaskPriority } from '@/api/generated/model';
import { useBoardStore } from '@/stores/boardStore';

interface TaskFilterBarProps {
  members?: ProjectMember[];
}

const priorityLabels: Record<TaskPriority, string> = {
  [TaskPriority.low]: 'Low',
  [TaskPriority.medium]: 'Medium',
  [TaskPriority.high]: 'High',
  [TaskPriority.urgent]: 'Urgent',
};

export function TaskFilterBar({ members }: TaskFilterBarProps) {
  const searchText = useBoardStore((s) => s.searchText);
  const setSearchText = useBoardStore((s) => s.setSearchText);
  const assigneeFilter = useBoardStore((s) => s.assigneeFilter);
  const setAssigneeFilter = useBoardStore((s) => s.setAssigneeFilter);
  const priorityFilter = useBoardStore((s) => s.priorityFilter);
  const setPriorityFilter = useBoardStore((s) => s.setPriorityFilter);
  const clearFilters = useBoardStore((s) => s.clearFilters);
  const hasActive = useBoardStore((s) => s.hasActiveFilters);

  const [assigneeOpen, setAssigneeOpen] = useState(false);
  const [priorityOpen, setPriorityOpen] = useState(false);

  const toggleAssignee = (id: string) => {
    setAssigneeFilter(
      assigneeFilter.includes(id)
        ? assigneeFilter.filter((a) => a !== id)
        : [...assigneeFilter, id],
    );
  };

  const togglePriority = (p: TaskPriority) => {
    setPriorityFilter(
      priorityFilter.includes(p) ? priorityFilter.filter((x) => x !== p) : [...priorityFilter, p],
    );
  };

  return (
    <div className="flex items-center gap-3">
      <input
        type="text"
        placeholder="Search tasks..."
        value={searchText}
        onChange={(e) => setSearchText(e.target.value)}
        className="rounded-md border border-gray-300 px-3 py-1.5 text-sm focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
      />

      <div className="relative">
        <button
          type="button"
          onClick={() => {
            setAssigneeOpen(!assigneeOpen);
            setPriorityOpen(false);
          }}
          className="rounded-md border border-gray-300 px-3 py-1.5 text-sm hover:bg-gray-50"
        >
          Assignee
        </button>
        {assigneeOpen && (
          <div
            data-testid="assignee-dropdown"
            className="absolute left-0 top-full z-10 mt-1 w-48 rounded-md border border-gray-200 bg-white p-2 shadow-lg"
          >
            <label className="flex items-center gap-2 rounded px-2 py-1 text-sm hover:bg-gray-50">
              <input
                type="checkbox"
                checked={assigneeFilter.includes('unassigned')}
                onChange={() => toggleAssignee('unassigned')}
              />
              Unassigned
            </label>
            {members?.map((m) => (
              <label
                key={m.user_id}
                className="flex items-center gap-2 rounded px-2 py-1 text-sm hover:bg-gray-50"
              >
                <input
                  type="checkbox"
                  checked={assigneeFilter.includes(m.user_id)}
                  onChange={() => toggleAssignee(m.user_id)}
                />
                {m.user_name}
              </label>
            ))}
          </div>
        )}
      </div>

      <div className="relative">
        <button
          type="button"
          onClick={() => {
            setPriorityOpen(!priorityOpen);
            setAssigneeOpen(false);
          }}
          className="rounded-md border border-gray-300 px-3 py-1.5 text-sm hover:bg-gray-50"
        >
          Priority
        </button>
        {priorityOpen && (
          <div
            data-testid="priority-dropdown"
            className="absolute left-0 top-full z-10 mt-1 w-48 rounded-md border border-gray-200 bg-white p-2 shadow-lg"
          >
            {Object.values(TaskPriority).map((p) => (
              <label
                key={p}
                className="flex items-center gap-2 rounded px-2 py-1 text-sm hover:bg-gray-50"
              >
                <input
                  type="checkbox"
                  checked={priorityFilter.includes(p)}
                  onChange={() => togglePriority(p)}
                />
                {priorityLabels[p]}
              </label>
            ))}
          </div>
        )}
      </div>

      {hasActive() && (
        <button
          type="button"
          onClick={clearFilters}
          className="rounded-md px-2 py-1 text-sm text-gray-500 hover:bg-gray-100 hover:text-gray-700"
        >
          Clear all
        </button>
      )}
    </div>
  );
}
