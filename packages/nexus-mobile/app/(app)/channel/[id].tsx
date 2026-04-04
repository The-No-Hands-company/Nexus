import { useEffect, useRef, useState, useCallback } from "react";
import {
  View, Text, TextInput, Pressable, FlatList,
  StyleSheet, KeyboardAvoidingView, Platform,
  ActivityIndicator, Alert,
} from "react-native";
import { useLocalSearchParams, Stack } from "expo-router";
import { format, isToday, isYesterday } from "date-fns";
import { useStore } from "../../src/store";
import type { NxMessage } from "../../src/types";

// ── Helpers ───────────────────────────────────────────────────────────────────

function formatTime(iso: string): string {
  const d = new Date(iso);
  if (isToday(d)) return `Today ${format(d, "h:mm a")}`;
  if (isYesterday(d)) return `Yesterday ${format(d, "h:mm a")}`;
  return format(d, "MMM d, h:mm a");
}

function isSameGroup(a: NxMessage, b: NxMessage): boolean {
  // Group consecutive messages from the same author within 5 minutes
  if (a.author_id !== b.author_id) return false;
  const diff = Math.abs(new Date(b.created_at).getTime() - new Date(a.created_at).getTime());
  return diff < 5 * 60 * 1000;
}

// ── Message bubble ────────────────────────────────────────────────────────────

function MessageBubble({
  msg,
  isFirst,
  isOwn,
  onReply,
  onDelete,
}: {
  msg: NxMessage;
  isFirst: boolean;
  isOwn: boolean;
  onReply: (msg: NxMessage) => void;
  onDelete: (msg: NxMessage) => void;
}) {
  return (
    <Pressable
      style={[ms.wrap, !isFirst && ms.wrapGrouped]}
      onLongPress={() => {
        if (isOwn) {
          Alert.alert("Message", "What would you like to do?", [
            { text: "Reply", onPress: () => onReply(msg) },
            { text: "Delete", style: "destructive", onPress: () => onDelete(msg) },
            { text: "Cancel", style: "cancel" },
          ]);
        } else {
          Alert.alert("Message", "What would you like to do?", [
            { text: "Reply", onPress: () => onReply(msg) },
            { text: "Cancel", style: "cancel" },
          ]);
        }
      }}
    >
      {isFirst ? (
        <View style={ms.header}>
          <View style={ms.avatar}>
            <Text style={ms.avatarText}>
              {(msg.author_username ?? "?").slice(0, 2).toUpperCase()}
            </Text>
          </View>
          <View style={ms.headerRight}>
            <Text style={[ms.username, isOwn && ms.usernameOwn]}>
              {msg.author_username ?? "Unknown"}
            </Text>
            <Text style={ms.time}>{formatTime(msg.created_at)}</Text>
          </View>
        </View>
      ) : (
        <View style={ms.indentPlaceholder} />
      )}

      <View style={ms.contentRow}>
        <Text style={ms.content}>{msg.content}</Text>
        {msg.edited && <Text style={ms.editedBadge}>(edited)</Text>}
      </View>

      {/* Attachments — images */}
      {msg.attachments.filter((a) => a.content_type?.startsWith("image/")).map((att) => (
        <View key={att.id} style={ms.attachment}>
          <Text style={ms.attachmentName}>{att.filename}</Text>
        </View>
      ))}

      {/* Reactions */}
      {msg.reactions.length > 0 && (
        <View style={ms.reactions}>
          {msg.reactions.map((r) => (
            <View key={r.emoji} style={[ms.reaction, r.me && ms.reactionOwn]}>
              <Text style={ms.reactionText}>{r.emoji} {r.count}</Text>
            </View>
          ))}
        </View>
      )}
    </Pressable>
  );
}

const ms = StyleSheet.create({
  wrap: { paddingTop: 14, paddingHorizontal: 12, paddingBottom: 2 },
  wrapGrouped: { paddingTop: 1 },
  header: { flexDirection: "row", gap: 10, marginBottom: 4 },
  avatar: {
    width: 36, height: 36, borderRadius: 18,
    backgroundColor: "rgba(39,201,165,0.2)",
    alignItems: "center", justifyContent: "center", flexShrink: 0,
  },
  avatarText: { color: "#27c9a5", fontWeight: "700", fontSize: 11 },
  headerRight: { flex: 1, flexDirection: "row", alignItems: "baseline", gap: 8 },
  username: { color: "#d6f4ff", fontWeight: "600", fontSize: 14 },
  usernameOwn: { color: "#27c9a5" },
  time: { color: "#556680", fontSize: 11 },
  indentPlaceholder: { width: 36, marginRight: 10 },
  contentRow: {
    marginLeft: 46, flexDirection: "row", alignItems: "flex-end",
    flexWrap: "wrap", gap: 4,
  },
  content: { color: "#c8e6f5", fontSize: 15, lineHeight: 21, flex: 1 },
  editedBadge: { color: "#556680", fontSize: 11 },
  attachment: {
    marginLeft: 46, marginTop: 4,
    backgroundColor: "#0b1a30", borderRadius: 8, padding: 8,
  },
  attachmentName: { color: "#7890b0", fontSize: 13 },
  reactions: {
    marginLeft: 46, marginTop: 4,
    flexDirection: "row", flexWrap: "wrap", gap: 4,
  },
  reaction: {
    backgroundColor: "#0b1a30", borderRadius: 12, paddingHorizontal: 8,
    paddingVertical: 3, borderWidth: 1, borderColor: "rgba(255,255,255,0.06)",
  },
  reactionOwn: { borderColor: "#27c9a5", backgroundColor: "rgba(39,201,165,0.1)" },
  reactionText: { color: "#9ec5d5", fontSize: 13 },
});

// ── Main screen ───────────────────────────────────────────────────────────────

export default function ChannelScreen() {
  const { id } = useLocalSearchParams<{ id: string }>();
  const {
    session, channels, dmChannels, messages, hasMoreMessages,
    loadMessages, loadMoreMessages, sendMessage, deleteMessage,
    markRead, sendTyping, typing, unread,
  } = useStore();

  const [draft, setDraft] = useState("");
  const [replyTo, setReplyTo] = useState<NxMessage | null>(null);
  const [sending, setSending] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const flatListRef = useRef<FlatList>(null);
  const typingTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const channel =
    channels.find((c) => c.id === id) ??
    dmChannels.find((c) => c.id === id);

  const channelMessages = messages[id ?? ""] ?? [];
  const typingUsers = typing[id ?? ""] ?? [];

  // Derive channel title
  const title = (() => {
    if (!channel) return "Channel";
    if (channel.kind === "dm" || channel.kind === "group_dm") {
      return (
        channel.recipients
          ?.filter((r) => r.id !== session?.userId)
          .map((r) => r.username)
          .join(", ") ?? "DM"
      );
    }
    return `#${channel.name}`;
  })();

  useEffect(() => {
    if (!id || !session) return;
    loadMessages(id);
    markRead(id);
  }, [id, session?.accessToken]);

  // Scroll to bottom on new messages
  useEffect(() => {
    if (channelMessages.length > 0) {
      setTimeout(() => flatListRef.current?.scrollToEnd({ animated: true }), 80);
    }
  }, [channelMessages.length]);

  const send = useCallback(async () => {
    if (!draft.trim() || !id) return;
    const content = draft.trim();
    setDraft("");
    setReplyTo(null);
    setSending(true);
    try {
      await sendMessage(id, content, replyTo?.id ?? null);
    } catch (e) {
      console.warn("[channel] send failed:", e);
      setDraft(content); // restore on failure
    } finally {
      setSending(false);
    }
  }, [draft, id, replyTo]);

  const handleDelete = useCallback(async (msg: NxMessage) => {
    if (!id) return;
    try {
      await deleteMessage(id, msg.id);
    } catch (e) {
      console.warn("[channel] delete failed:", e);
    }
  }, [id]);

  const handleTyping = useCallback(() => {
    if (!id) return;
    if (typingTimerRef.current) clearTimeout(typingTimerRef.current);
    sendTyping(id);
    typingTimerRef.current = setTimeout(() => {}, 4_000);
  }, [id]);

  const loadMore = useCallback(async () => {
    if (!id || loadingMore || !hasMoreMessages[id]) return;
    setLoadingMore(true);
    await loadMoreMessages(id);
    setLoadingMore(false);
  }, [id, loadingMore, hasMoreMessages]);

  const renderItem = useCallback(
    ({ item, index }: { item: NxMessage; index: number }) => {
      const prev = channelMessages[index - 1];
      const isFirst = !prev || !isSameGroup(prev, item);
      const isOwn = item.author_id === session?.userId;
      return (
        <MessageBubble
          msg={item}
          isFirst={isFirst}
          isOwn={isOwn}
          onReply={setReplyTo}
          onDelete={handleDelete}
        />
      );
    },
    [channelMessages, session?.userId, handleDelete]
  );

  return (
    <KeyboardAvoidingView
      style={s.root}
      behavior={Platform.OS === "ios" ? "padding" : "height"}
      keyboardVerticalOffset={Platform.OS === "ios" ? 88 : 0}
    >
      <Stack.Screen options={{ title }} />

      {/* Load more button */}
      {hasMoreMessages[id ?? ""] && (
        <Pressable style={s.loadMore} onPress={loadMore}>
          {loadingMore ? (
            <ActivityIndicator size="small" color="#27c9a5" />
          ) : (
            <Text style={s.loadMoreText}>Load older messages</Text>
          )}
        </Pressable>
      )}

      <FlatList
        ref={flatListRef}
        data={channelMessages}
        keyExtractor={(m) => m.id}
        renderItem={renderItem}
        contentContainerStyle={s.list}
        removeClippedSubviews
        maxToRenderPerBatch={20}
        windowSize={10}
        onEndReachedThreshold={0.3}
        keyboardShouldPersistTaps="handled"
      />

      {/* Typing indicator */}
      {typingUsers.length > 0 && (
        <View style={s.typingBar}>
          <Text style={s.typingText}>
            {typingUsers.length === 1
              ? `${typingUsers[0]} is typing…`
              : typingUsers.length === 2
              ? `${typingUsers[0]} and ${typingUsers[1]} are typing…`
              : "Several people are typing…"}
          </Text>
        </View>
      )}

      {/* Reply preview */}
      {replyTo && (
        <View style={s.replyBar}>
          <Text style={s.replyLabel}>Replying to {replyTo.author_username}</Text>
          <Text style={s.replyPreview} numberOfLines={1}>
            {replyTo.content}
          </Text>
          <Pressable onPress={() => setReplyTo(null)} style={s.replyClose}>
            <Text style={s.replyCloseText}>✕</Text>
          </Pressable>
        </View>
      )}

      {/* Input bar */}
      <View style={s.inputBar}>
        <TextInput
          style={s.input}
          value={draft}
          onChangeText={(t) => { setDraft(t); handleTyping(); }}
          placeholder={title ? `Message ${title}` : "Message…"}
          placeholderTextColor="#556680"
          multiline
          maxLength={4000}
          onSubmitEditing={send}
          blurOnSubmit={false}
        />
        <Pressable
          style={[s.sendBtn, (!draft.trim() || sending) && s.sendBtnDisabled]}
          onPress={send}
          disabled={!draft.trim() || sending}
        >
          {sending ? (
            <ActivityIndicator size="small" color="#062020" />
          ) : (
            <Text style={s.sendIcon}>↑</Text>
          )}
        </Pressable>
      </View>
    </KeyboardAvoidingView>
  );
}

const s = StyleSheet.create({
  root: { flex: 1, backgroundColor: "#030a14" },
  list: { paddingBottom: 8 },
  loadMore: {
    alignItems: "center", paddingVertical: 10,
    backgroundColor: "#071327",
    borderBottomWidth: 1, borderBottomColor: "rgba(255,255,255,0.04)",
  },
  loadMoreText: { color: "#27c9a5", fontSize: 13 },
  typingBar: { paddingHorizontal: 16, paddingVertical: 4 },
  typingText: { color: "#7890b0", fontSize: 12, fontStyle: "italic" },
  replyBar: {
    flexDirection: "row", alignItems: "center",
    backgroundColor: "#0b1a30", paddingHorizontal: 12, paddingVertical: 8,
    borderTopWidth: 1, borderTopColor: "rgba(255,255,255,0.06)",
    gap: 8,
  },
  replyLabel: { color: "#27c9a5", fontWeight: "600", fontSize: 12 },
  replyPreview: { flex: 1, color: "#7890b0", fontSize: 12 },
  replyClose: { padding: 4 },
  replyCloseText: { color: "#556680", fontSize: 14 },
  inputBar: {
    flexDirection: "row", alignItems: "flex-end", gap: 8,
    paddingHorizontal: 12, paddingVertical: 10,
    backgroundColor: "#071327",
    borderTopWidth: 1, borderTopColor: "rgba(255,255,255,0.06)",
  },
  input: {
    flex: 1, backgroundColor: "#0b1a30", color: "#d6f4ff",
    borderRadius: 20, paddingHorizontal: 16, paddingVertical: 10,
    fontSize: 15, maxHeight: 120,
    borderWidth: 1, borderColor: "rgba(255,255,255,0.08)",
  },
  sendBtn: {
    width: 40, height: 40, borderRadius: 20,
    backgroundColor: "#27c9a5",
    alignItems: "center", justifyContent: "center",
  },
  sendBtnDisabled: { backgroundColor: "#112544" },
  sendIcon: { color: "#062020", fontSize: 20, fontWeight: "700", lineHeight: 22 },
});
