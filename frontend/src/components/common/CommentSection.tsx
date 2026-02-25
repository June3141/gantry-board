import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  getListCommentsQueryKey,
  useCreateComment,
  useDeleteComment,
  useListComments,
  useUpdateComment,
} from '@/api/generated/endpoints/task-comments/task-comments';
import { useAuthStore } from '@/stores/authStore';
import { useToastStore } from '@/stores/toastStore';

function timeAgo(
  dateStr: string,
  t: (key: string, opts?: Record<string, unknown>) => string,
): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diffSec = Math.floor((now - then) / 1000);
  if (diffSec < 60) return t('common.timeAgo.justNow');
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return t('common.timeAgo.minutesAgo', { count: diffMin });
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return t('common.timeAgo.hoursAgo', { count: diffHr });
  const diffDay = Math.floor(diffHr / 24);
  return t('common.timeAgo.daysAgo', { count: diffDay });
}

function CommentItem({
  comment,
  taskId,
  isOwner,
}: {
  comment: { id: string; user_name: string; user_id: string; content: string; created_at: string };
  taskId: string;
  isOwner: boolean;
}) {
  const { t } = useTranslation();
  const [editing, setEditing] = useState(false);
  const [editValue, setEditValue] = useState('');
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const queryClient = useQueryClient();
  const updateComment = useUpdateComment();
  const deleteComment = useDeleteComment();
  const addToast = useToastStore((s) => s.addToast);

  const invalidateComments = () => {
    queryClient.invalidateQueries({ queryKey: getListCommentsQueryKey(taskId) });
  };

  const handleEdit = () => {
    setEditing(true);
    setEditValue(comment.content);
  };

  const handleSave = async () => {
    const trimmed = editValue.trim();
    if (!trimmed) return;
    try {
      await updateComment.mutateAsync({
        taskId,
        commentId: comment.id,
        data: { content: trimmed },
      });
      setEditing(false);
      invalidateComments();
    } catch {
      addToast('error', t('comment.updateFailed'));
    }
  };

  const handleDelete = async () => {
    try {
      await deleteComment.mutateAsync({ taskId, commentId: comment.id });
      setShowDeleteConfirm(false);
      invalidateComments();
    } catch {
      addToast('error', t('comment.deleteFailed'));
    }
  };

  const initials = comment.user_name
    .split(' ')
    .map((w) => w[0])
    .join('')
    .toUpperCase()
    .slice(0, 2);

  return (
    <div data-testid="comment-item" className="flex gap-3 py-2">
      <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-gray-200 text-xs font-medium text-gray-600">
        {initials}
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-gray-900">{comment.user_name}</span>
          <span className="text-xs text-gray-500">{timeAgo(comment.created_at, t)}</span>
          {isOwner && !editing && !showDeleteConfirm && (
            <div className="ml-auto flex gap-1">
              <button
                type="button"
                aria-label={t('common.edit')}
                onClick={handleEdit}
                className="text-xs text-gray-400 hover:text-gray-600"
              >
                {t('common.edit')}
              </button>
              <button
                type="button"
                aria-label={t('common.delete')}
                onClick={() => setShowDeleteConfirm(true)}
                className="text-xs text-gray-400 hover:text-red-600"
              >
                {t('common.delete')}
              </button>
            </div>
          )}
        </div>
        {editing ? (
          <div className="mt-1 space-y-1">
            <textarea
              value={editValue}
              onChange={(e) => setEditValue(e.target.value)}
              rows={2}
              className="block w-full rounded border border-gray-300 px-2 py-1 text-sm focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
            />
            <div className="flex gap-1">
              <button
                type="button"
                onClick={handleSave}
                className="rounded bg-blue-600 px-2 py-0.5 text-xs text-white hover:bg-blue-700"
              >
                {t('common.save')}
              </button>
              <button
                type="button"
                onClick={() => setEditing(false)}
                className="rounded border border-gray-300 px-2 py-0.5 text-xs text-gray-700 hover:bg-gray-50"
              >
                {t('common.cancel')}
              </button>
            </div>
          </div>
        ) : showDeleteConfirm ? (
          <div className="mt-1 flex items-center gap-2 rounded bg-red-50 px-2 py-1">
            <span className="text-xs text-red-700">{t('comment.deleteConfirm')}</span>
            <button
              type="button"
              onClick={handleDelete}
              className="rounded bg-red-600 px-2 py-0.5 text-xs text-white hover:bg-red-700"
            >
              {t('common.confirm')}
            </button>
            <button
              type="button"
              onClick={() => setShowDeleteConfirm(false)}
              className="rounded border border-gray-300 px-2 py-0.5 text-xs text-gray-700 hover:bg-gray-50"
            >
              {t('common.cancel')}
            </button>
          </div>
        ) : (
          <p className="mt-0.5 text-sm text-gray-700">{comment.content}</p>
        )}
      </div>
    </div>
  );
}

export function CommentSection({ taskId }: { taskId: string }) {
  const { t } = useTranslation();
  const { data: comments, isLoading } = useListComments(taskId);
  const createComment = useCreateComment();
  const queryClient = useQueryClient();
  const [newComment, setNewComment] = useState('');
  const currentUser = useAuthStore((s) => s.user);
  const addToast = useToastStore((s) => s.addToast);

  const handleSubmit = async () => {
    const trimmed = newComment.trim();
    if (!trimmed) return;
    try {
      await createComment.mutateAsync({ taskId, data: { content: trimmed } });
      setNewComment('');
      queryClient.invalidateQueries({ queryKey: getListCommentsQueryKey(taskId) });
    } catch {
      addToast('error', t('comment.postFailed'));
    }
  };

  return (
    <div>
      {isLoading ? (
        <p className="text-sm text-gray-500">{t('comment.loadingComments')}</p>
      ) : comments && comments.length > 0 ? (
        <div className="divide-y">
          {comments.map((c) => (
            <CommentItem
              key={c.id}
              comment={c}
              taskId={taskId}
              isOwner={currentUser?.id === c.user_id}
            />
          ))}
        </div>
      ) : (
        <p className="text-sm text-gray-500">{t('comment.noComments')}</p>
      )}
      <div className="mt-3 flex gap-2">
        <textarea
          value={newComment}
          onChange={(e) => setNewComment(e.target.value)}
          placeholder={t('comment.addPlaceholder')}
          rows={2}
          className="flex-1 rounded border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
        />
        <button
          type="button"
          onClick={handleSubmit}
          disabled={!newComment.trim() || createComment.isPending}
          className="self-end rounded bg-blue-600 px-3 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
        >
          {t('common.post')}
        </button>
      </div>
    </div>
  );
}
