import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  getListCommentsQueryKey,
  useDeleteComment,
  useUpdateComment,
} from '@/api/generated/endpoints/task-comments/task-comments';
import type { TaskComment } from '@/api/generated/model';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { useToastStore } from '@/stores/toastStore';
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
      addToast('error', t('activity.commentUpdateFailed'));
    }
  };

  const handleDelete = async () => {
    try {
      await deleteComment.mutateAsync({ taskId, commentId: comment.id });
      setShowDeleteConfirm(false);
      invalidateComments();
    } catch {
      addToast('error', t('activity.commentDeleteFailed'));
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
              <Button
                variant="ghost"
                size="xs"
                aria-label={t('common.edit')}
                onClick={() => {
                  setEditing(true);
                  setEditValue(comment.content);
                }}
                className="text-muted-foreground"
              >
                {t('common.edit')}
              </Button>
              <Button
                variant="ghost"
                size="xs"
                aria-label={t('common.delete')}
                onClick={() => setShowDeleteConfirm(true)}
                className="text-muted-foreground hover:text-destructive"
              >
                {t('common.delete')}
              </Button>
            </div>
          )}
        </div>
        {editing ? (
          <div className="mt-1 space-y-1">
            <Textarea
              aria-label={t('task.editComment')}
              value={editValue}
              onChange={(e) => setEditValue(e.target.value)}
              rows={2}
            />
            <div className="flex gap-1">
              <Button size="xs" onClick={handleSave}>
                {t('common.save')}
              </Button>
              <Button variant="outline" size="xs" onClick={() => setEditing(false)}>
                {t('common.cancel')}
              </Button>
            </div>
          </div>
        ) : showDeleteConfirm ? (
          <div className="mt-1 flex items-center gap-2 rounded bg-red-50 px-2 py-1">
            <span className="text-xs text-red-700">{t('activity.deleteCommentConfirm')}</span>
            <Button variant="destructive" size="xs" onClick={handleDelete}>
              {t('common.confirm')}
            </Button>
            <Button variant="outline" size="xs" onClick={() => setShowDeleteConfirm(false)}>
              {t('common.cancel')}
            </Button>
          </div>
        ) : (
          <p className="mt-0.5 text-sm text-gray-700">{comment.content}</p>
        )}
      </div>
    </div>
  );
}
