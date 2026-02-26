import { useTranslation } from 'react-i18next';

export type ActivityFilterValue = 'all' | 'comments';

export function ActivityFilter({
  value,
  onChange,
}: {
  value: ActivityFilterValue;
  onChange: (value: ActivityFilterValue) => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="inline-flex rounded-md border border-border">
      <button
        type="button"
        aria-pressed={value === 'all'}
        onClick={() => {
          if (value !== 'all') onChange('all');
        }}
        className={`rounded-l-md px-3 py-1 text-xs font-medium ${
          value === 'all'
            ? 'bg-muted text-foreground'
            : 'bg-background text-muted-foreground hover:bg-accent'
        }`}
      >
        {t('activity.filterAll')}
      </button>
      <button
        type="button"
        aria-pressed={value === 'comments'}
        onClick={() => {
          if (value !== 'comments') onChange('comments');
        }}
        className={`rounded-r-md px-3 py-1 text-xs font-medium ${
          value === 'comments'
            ? 'bg-muted text-foreground'
            : 'bg-background text-muted-foreground hover:bg-accent'
        }`}
      >
        {t('activity.filterComments')}
      </button>
    </div>
  );
}
