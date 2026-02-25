import { useQueryClient } from '@tanstack/react-query';
import { X } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  getListMessagesQueryKey,
  useCreateMessage,
  useDeleteMessage,
  useListMessages,
} from '@/api/generated/endpoints/project-messages/project-messages';
import { useEscapeKey } from '@/hooks/useEscapeKey';
import { useAuthStore } from '@/stores/authStore';
import { useToastStore } from '@/stores/toastStore';
import { useUiStore } from '@/stores/uiStore';

function timeAgo(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diffSec = Math.floor((now - then) / 1000);
  if (diffSec < 60) return 'just now';
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `${diffMin} min ago`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDay = Math.floor(diffHr / 24);
  return `${diffDay}d ago`;
}

function MessageItem({
  message,
  projectId,
  isOwner,
}: {
  message: {
    id: string;
    user_name: string;
    user_id: string;
    content: string;
    created_at: string;
  };
  projectId: string;
  isOwner: boolean;
}) {
  const { t } = useTranslation();
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const queryClient = useQueryClient();
  const deleteMessage = useDeleteMessage();
  const addToast = useToastStore((s) => s.addToast);

  const invalidateMessages = () => {
    queryClient.invalidateQueries({ queryKey: getListMessagesQueryKey(projectId) });
  };

  const handleDelete = async () => {
    try {
      await deleteMessage.mutateAsync({ projectId, messageId: message.id });
      setShowDeleteConfirm(false);
      invalidateMessages();
    } catch {
      addToast('error', t('chat.deleteFailed'));
    }
  };

  const initials = message.user_name
    .split(' ')
    .map((w) => w[0])
    .join('')
    .toUpperCase()
    .slice(0, 2);

  return (
    <div data-testid="message-item" className="flex gap-3 py-2">
      <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-blue-100 text-xs font-medium text-blue-700">
        {initials}
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-gray-900">{message.user_name}</span>
          <span className="text-xs text-gray-500">{timeAgo(message.created_at)}</span>
          {isOwner && !showDeleteConfirm && (
            <button
              type="button"
              aria-label={t('common.delete')}
              onClick={() => setShowDeleteConfirm(true)}
              className="ml-auto text-xs text-gray-400 hover:text-red-600"
            >
              {t('common.delete')}
            </button>
          )}
        </div>
        {showDeleteConfirm ? (
          <div className="mt-1 flex items-center gap-2 rounded bg-red-50 px-2 py-1">
            <span className="text-xs text-red-700">{t('chat.deleteMessageConfirm')}</span>
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
          <p className="mt-0.5 text-sm text-gray-700">{message.content}</p>
        )}
      </div>
    </div>
  );
}

export function ProjectChatPanel({ projectId }: { projectId: string }) {
  const isOpen = useUiStore((s) => s.isProjectChatOpen);
  if (!isOpen) return null;
  return <ProjectChatContent projectId={projectId} />;
}

function ProjectChatContent({ projectId }: { projectId: string }) {
  const { t } = useTranslation();
  const closeProjectChat = useUiStore((s) => s.closeProjectChat);
  const { data: messages, isLoading } = useListMessages(projectId);
  const createMessage = useCreateMessage();
  const queryClient = useQueryClient();
  const [newMessage, setNewMessage] = useState('');
  const currentUser = useAuthStore((s) => s.user);
  const addToast = useToastStore((s) => s.addToast);

  useEscapeKey(closeProjectChat);

  const handleSubmit = async () => {
    const trimmed = newMessage.trim();
    if (!trimmed) return;
    try {
      await createMessage.mutateAsync({
        projectId,
        data: { content: trimmed },
      });
      setNewMessage('');
      queryClient.invalidateQueries({ queryKey: getListMessagesQueryKey(projectId) });
    } catch {
      addToast('error', t('chat.sendFailed'));
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  // Messages come in DESC order from API; reverse for display (oldest first)
  const displayMessages = messages ? [...messages].reverse() : [];

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="project-chat-title"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onClick={(e) => {
        if (e.target === e.currentTarget) closeProjectChat();
      }}
    >
      <div className="flex h-[600px] w-full max-w-lg flex-col rounded-lg bg-white shadow-xl">
        <div className="flex items-center justify-between border-b px-4 py-3">
          <h2 id="project-chat-title" className="text-lg font-semibold text-gray-900">
            {t('chat.projectChat')}
          </h2>
          <button
            type="button"
            onClick={closeProjectChat}
            className="text-gray-400 hover:text-gray-600"
            aria-label={t('common.close')}
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto px-4 py-2">
          {isLoading ? (
            <p className="text-sm text-gray-500">{t('chat.loadingMessages')}</p>
          ) : displayMessages.length > 0 ? (
            <div className="divide-y">
              {displayMessages.map((m) => (
                <MessageItem
                  key={m.id}
                  message={m}
                  projectId={projectId}
                  isOwner={currentUser?.id === m.user_id}
                />
              ))}
            </div>
          ) : (
            <p className="py-8 text-center text-sm text-gray-500">
              {t('chat.noMessages')}
            </p>
          )}
        </div>

        <div className="border-t px-4 py-3">
          <div className="flex gap-2">
            <textarea
              value={newMessage}
              onChange={(e) => setNewMessage(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={t('chat.placeholder')}
              rows={2}
              className="flex-1 rounded border border-gray-300 px-3 py-2 text-sm focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
            />
            <button
              type="button"
              aria-label={t('common.send')}
              onClick={handleSubmit}
              disabled={!newMessage.trim() || createMessage.isPending}
              className="self-end rounded bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
            >
              {t('common.send')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
