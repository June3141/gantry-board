import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import {
  getListCommentsQueryKey,
  useDeleteComment,
  useUpdateComment,
} from '../api/generated/endpoints/task-comments/task-comments';
import type { TaskComment } from '../api/generated/model';
import { useToastStore } from '../stores/toastStore';
import { getInitials, timeAgo } from './timelineUtils';

export function TimelineCommentItem({
  comment,
  taskId,
  isOwner,
}: {
  comment: TaskComment;
  taskId: string;
  isOwner: boolean;
}) {
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
      addToast('error', 'Failed to update comment.');
    }
  };

  const handleDelete = async () => {
    try {
      await deleteComment.mutateAsync({ taskId, commentId: comment.id });
      setShowDeleteConfirm(false);
      invalidateComments();
    } catch {
      addToast('error', 'Failed to delete comment.');
    }
  };

  return (
    <div data-testid="timeline-comment" className="flex gap-3 py-2">
      <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-gray-200 text-xs font-medium text-gray-600">
        {getInitials(comment.user_name)}
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-gray-900">{comment.user_name}</span>
          <span className="text-xs text-gray-500">{timeAgo(comment.created_at)}</span>
          {isOwner && !editing && !showDeleteConfirm && (
            <div className="ml-auto flex gap-1">
              <button
                type="button"
                aria-label="Edit"
                onClick={() => {
                  setEditing(true);
                  setEditValue(comment.content);
                }}
                className="text-xs text-gray-400 hover:text-gray-600"
              >
                Edit
              </button>
              <button
                type="button"
                aria-label="Delete"
                onClick={() => setShowDeleteConfirm(true)}
                className="text-xs text-gray-400 hover:text-red-600"
              >
                Delete
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
                Save
              </button>
              <button
                type="button"
                onClick={() => setEditing(false)}
                className="rounded border border-gray-300 px-2 py-0.5 text-xs text-gray-700 hover:bg-gray-50"
              >
                Cancel
              </button>
            </div>
          </div>
        ) : showDeleteConfirm ? (
          <div className="mt-1 flex items-center gap-2 rounded bg-red-50 px-2 py-1">
            <span className="text-xs text-red-700">Delete this comment?</span>
            <button
              type="button"
              onClick={handleDelete}
              className="rounded bg-red-600 px-2 py-0.5 text-xs text-white hover:bg-red-700"
            >
              Confirm
            </button>
            <button
              type="button"
              onClick={() => setShowDeleteConfirm(false)}
              className="rounded border border-gray-300 px-2 py-0.5 text-xs text-gray-700 hover:bg-gray-50"
            >
              Cancel
            </button>
          </div>
        ) : (
          <p className="mt-0.5 text-sm text-gray-700">{comment.content}</p>
        )}
      </div>
    </div>
  );
}
