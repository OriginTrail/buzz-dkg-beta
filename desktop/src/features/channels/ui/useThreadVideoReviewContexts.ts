import * as React from "react";

import { buildVideoReviewContextsByMessageId } from "@/features/messages/lib/videoReviewContext";
import type { TimelineMessage } from "@/features/messages/types";
import type { ChannelPaneProps } from "./ChannelPane.types";

type ThreadVideoReviewContextArgs = Pick<
  ChannelPaneProps,
  | "activeChannel"
  | "isSending"
  | "messages"
  | "onSendVideoReviewComment"
  | "onToggleReaction"
  | "profiles"
  | "threadAllMessages"
  | "threadHeadMessage"
>;

export function useThreadVideoReviewContexts({
  activeChannel,
  isSending,
  messages,
  onSendVideoReviewComment,
  onToggleReaction,
  profiles,
  threadAllMessages,
  threadHeadMessage,
}: ThreadVideoReviewContextArgs) {
  const activeSender = activeChannel?.archivedAt
    ? undefined
    : onSendVideoReviewComment;

  return React.useMemo(() => {
    const messagesById = new Map<string, TimelineMessage>(
      messages.map((message) => [message.id, message]),
    );
    if (threadHeadMessage) {
      messagesById.set(threadHeadMessage.id, threadHeadMessage);
    }
    for (const message of threadAllMessages) {
      messagesById.set(message.id, message);
    }
    return buildVideoReviewContextsByMessageId({
      channelId: activeChannel?.id ?? null,
      channelName: activeChannel?.name,
      channelType: activeChannel?.channelType ?? null,
      isSendingVideoReviewComment: isSending,
      messages: [...messagesById.values()],
      onSendVideoReviewComment: activeSender,
      onToggleReaction,
      profiles,
    });
  }, [
    activeChannel,
    activeSender,
    isSending,
    messages,
    onToggleReaction,
    profiles,
    threadAllMessages,
    threadHeadMessage,
  ]);
}
