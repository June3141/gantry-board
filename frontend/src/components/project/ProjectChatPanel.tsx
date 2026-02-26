import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  getListMessagesQueryKey,
  useCreateMessage,
  useDeleteMessage,
  useListMessages,
} from '@/api/generated/endpoints/project-messages/project-messages';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Textarea } from '@/components/ui/textarea';
import { useAuthStore } from '@/stores/authStore';
import { useToastStore } from '@/stores/toastStore';
import { useUiStore } from '@/stores/uiStore';

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
      <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-primary/15 text-xs font-medium text-primary">
        {initials}
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-foreground">{message.user_name}</span>
          <span className="text-xs text-muted-foreground">{timeAgo(message.created_at, t)}</span>
          {isOwner && !showDeleteConfirm && (
            <Button
              variant="ghost"
              size="xs"
              aria-label={t('common.delete')}
              onClick={() => setShowDeleteConfirm(true)}
              className="ml-auto text-muted-foreground hover:text-destructive"
            >
              {t('common.delete')}
            </Button>
          )}
        </div>
        {showDeleteConfirm ? (
          <div className="mt-1 flex items-center gap-2 rounded bg-destructive/10 px-2 py-1">
            <span className="text-xs text-destructive">{t('chat.deleteMessageConfirm')}</span>
            <Button variant="destructive" size="xs" onClick={handleDelete}>
              {t('common.confirm')}
            </Button>
            <Button variant="outline" size="xs" onClick={() => setShowDeleteConfirm(false)}>
              {t('common.cancel')}
            </Button>
          </div>
        ) : (
          <p className="mt-0.5 text-sm text-foreground">{message.content}</p>
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
    <Dialog
      open={true}
      onOpenChange={(open) => {
        if (!open) closeProjectChat();
      }}
    >
      <DialogContent
        className="flex h-[600px] max-w-lg flex-col gap-0 p-0"
        aria-describedby={undefined}
      >
        <DialogHeader className="border-b px-4 py-3">
          <DialogTitle>{t('chat.projectChat')}</DialogTitle>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto px-4 py-2">
          {isLoading ? (
            <p className="text-sm text-muted-foreground">{t('chat.loadingMessages')}</p>
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
            <p className="py-8 text-center text-sm text-muted-foreground">{t('chat.noMessages')}</p>
          )}
        </div>

        <div className="border-t px-4 py-3">
          <div className="flex gap-2">
            <Textarea
              value={newMessage}
              onChange={(e) => setNewMessage(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={t('chat.placeholder')}
              rows={2}
              className="flex-1"
            />
            <Button
              aria-label={t('common.send')}
              onClick={handleSubmit}
              disabled={!newMessage.trim() || createMessage.isPending}
              className="self-end"
            >
              {t('common.send')}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
