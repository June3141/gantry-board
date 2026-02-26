import { Search } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ProjectMember } from '@/api/generated/model';
import { TaskPriority } from '@/api/generated/model';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { useBoardStore } from '@/stores/boardStore';

interface TaskFilterBarProps {
  members?: ProjectMember[];
}

const priorityLabelKeys: Record<TaskPriority, string> = {
  [TaskPriority.low]: 'board.priorityLabel.low',
  [TaskPriority.medium]: 'board.priorityLabel.medium',
  [TaskPriority.high]: 'board.priorityLabel.high',
  [TaskPriority.urgent]: 'board.priorityLabel.urgent',
};

export function TaskFilterBar({ members }: TaskFilterBarProps) {
  const { t } = useTranslation();
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
      <div className="relative">
        <Search className="absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          type="text"
          placeholder={t('board.searchPlaceholder')}
          value={searchText}
          onChange={(e) => setSearchText(e.target.value)}
          className="pl-8 pr-3"
        />
      </div>

      <div className="relative">
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            setAssigneeOpen(!assigneeOpen);
            setPriorityOpen(false);
          }}
        >
          {t('board.assignee')}
        </Button>
        {assigneeOpen && (
          <div
            data-testid="assignee-dropdown"
            className="absolute left-0 top-full z-10 mt-1 w-48 rounded-md border border-border bg-background p-2 shadow-lg"
          >
            <label className="flex items-center gap-2 rounded px-2 py-1 text-sm hover:bg-accent">
              <input
                type="checkbox"
                checked={assigneeFilter.includes('unassigned')}
                onChange={() => toggleAssignee('unassigned')}
              />
              {t('common.unassigned')}
            </label>
            {members?.map((m) => (
              <label
                key={m.user_id}
                className="flex items-center gap-2 rounded px-2 py-1 text-sm hover:bg-accent"
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
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            setPriorityOpen(!priorityOpen);
            setAssigneeOpen(false);
          }}
        >
          {t('board.priority')}
        </Button>
        {priorityOpen && (
          <div
            data-testid="priority-dropdown"
            className="absolute left-0 top-full z-10 mt-1 w-48 rounded-md border border-border bg-background p-2 shadow-lg"
          >
            {Object.values(TaskPriority).map((p) => (
              <label
                key={p}
                className="flex items-center gap-2 rounded px-2 py-1 text-sm hover:bg-accent"
              >
                <input
                  type="checkbox"
                  checked={priorityFilter.includes(p)}
                  onChange={() => togglePriority(p)}
                />
                {t(priorityLabelKeys[p])}
              </label>
            ))}
          </div>
        )}
      </div>

      {hasActive() && (
        <Button variant="ghost" size="sm" onClick={clearFilters}>
          {t('common.clearAll')}
        </Button>
      )}
    </div>
  );
}
