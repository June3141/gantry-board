import { useListPullRequests } from '../api/generated/endpoints/pull-requests/pull-requests';
import type { GitHubPullRequest } from '../api/generated/model';

export function PullRequestList({ taskId }: { taskId: string }) {
  const { data: pullRequests, isLoading, isError } = useListPullRequests(taskId);

  if (isLoading) {
    return <p className="text-sm text-gray-500">Loading pull requests...</p>;
  }

  if (isError) {
    return <p className="text-sm text-red-500">Failed to load pull requests.</p>;
  }

  if (!pullRequests || pullRequests.length === 0) {
    return <p className="text-sm text-gray-500">No pull requests</p>;
  }

  return (
    <div className="space-y-1">
      {pullRequests.map((pr) => (
        <PullRequestItem key={pr.id} pr={pr} />
      ))}
    </div>
  );
}

function PullRequestItem({ pr }: { pr: GitHubPullRequest }) {
  const badge = getBadge(pr);

  return (
    <div className="flex items-center justify-between rounded-md border border-gray-200 px-3 py-2">
      <div className="flex items-center gap-2 text-sm min-w-0">
        <span className="text-gray-500 shrink-0">#{pr.pr_number}</span>
        <a
          href={pr.url}
          target="_blank"
          rel="noopener noreferrer"
          className="truncate font-medium text-blue-600 hover:underline"
        >
          {pr.title}
        </a>
        {pr.author && <span className="text-gray-400 shrink-0">{pr.author}</span>}
      </div>
      <span
        className={`inline-flex shrink-0 items-center rounded-full px-2 py-0.5 text-xs font-medium ${badge.classes}`}
      >
        {badge.label}
      </span>
    </div>
  );
}

function getBadge(pr: GitHubPullRequest): { label: string; classes: string } {
  if (pr.state === 'open') {
    return { label: 'open', classes: 'bg-green-100 text-green-800' };
  }
  if (pr.is_merged) {
    return { label: 'merged', classes: 'bg-purple-100 text-purple-800' };
  }
  return { label: 'closed', classes: 'bg-red-100 text-red-800' };
}
