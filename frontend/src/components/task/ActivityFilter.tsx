export type ActivityFilterValue = 'all' | 'comments';

export function ActivityFilter({
  value,
  onChange,
}: {
  value: ActivityFilterValue;
  onChange: (value: ActivityFilterValue) => void;
}) {
  return (
    <div className="inline-flex rounded-md border border-gray-200">
      <button
        type="button"
        aria-pressed={value === 'all'}
        onClick={() => {
          if (value !== 'all') onChange('all');
        }}
        className={`rounded-l-md px-3 py-1 text-xs font-medium ${
          value === 'all' ? 'bg-gray-100 text-gray-900' : 'bg-white text-gray-500 hover:bg-gray-50'
        }`}
      >
        All Activity
      </button>
      <button
        type="button"
        aria-pressed={value === 'comments'}
        onClick={() => {
          if (value !== 'comments') onChange('comments');
        }}
        className={`rounded-r-md px-3 py-1 text-xs font-medium ${
          value === 'comments'
            ? 'bg-gray-100 text-gray-900'
            : 'bg-white text-gray-500 hover:bg-gray-50'
        }`}
      >
        Comments Only
      </button>
    </div>
  );
}
