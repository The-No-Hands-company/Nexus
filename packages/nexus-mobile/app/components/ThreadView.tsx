/**
 * ThreadView.tsx - Thread discussion view
 */
import React, { useState, useEffect } from "react";
import {
  View, Text, FlatList, TextInput, Pressable, StyleSheet,
  SafeAreaView, KeyboardAvoidingView, Platform, ActivityIndicator
} from "react-native";
import { useLocalSearchParams, useRouter } from "expo-router";
import { store } from "../lib/store";

export default function ThreadView() {
  const { channelId, threadId } = useLocalSearchParams<{ channelId: string; threadId: string }>();
  const router = useRouter();
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [thread, setThread] = useState<any>(null);

  useEffect(() => {
    loadThread();
  }, [threadId]);

  async function loadThread() {
    try {
      const data = await store.api.threads.get(threadId);
      setThread(data);
    } catch (e) {
      console.error("Failed to load thread:", e);
    }
  }

  async function handleReply() {
    if (!input.trim() || loading) return;
    setLoading(true);
    try {
      await store.api.threads.reply(channelId, threadId, input.trim());
      setInput("");
      await loadThread();
    } catch (e: any) {
      alert(e.message);
    } finally {
      setLoading(false);
    }
  }

  return (
    <SafeAreaView style={styles.container}>
      {/* Header */}
      <View style={styles.header}>
        <Pressable onPress={() => router.back()} style={styles.backBtn}>
          <Text style={styles.backBtnText}>Back</Text>
        </Pressable>
        <View style={styles.headerTitleArea}>
          <Text style={styles.headerTitle} numberOfLines={1}>{thread?.title || "Thread"}</Text>
          <Text style={styles.headerMeta}>{thread?.replyCount || 0} replies</Text>
        </View>
      </View>

      {/* Parent message */}
      {thread?.parentMessage && (
        <View style={styles.parentMessage}>
          <View style={styles.avatar}>
            <Text style={styles.avatarText}>
              {thread.parentMessage.authorUsername[0]?.toUpperCase()}
            </Text>
          </View>
          <View style={styles.messageContent}>
            <Text style={styles.authorName}>{thread.parentMessage.authorUsername}</Text>
            <Text style={styles.messageText}>{thread.parentMessage.content}</Text>
          </View>
        </View>
      )}

      {/* Replies */}
      <FlatList
        data={thread?.replies || []}
        keyExtractor={item => item.id}
        renderItem={({ item }) => (
          <View style={styles.reply}>
            <View style={styles.avatar}>
              <Text style={styles.avatarText}>{item.authorUsername[0]?.toUpperCase()}</Text>
            </View>
            <View style={styles.messageContent}>
              <Text style={styles.authorName}>{item.authorUsername}</Text>
              <Text style={styles.messageText}>{item.content}</Text>
              <Text style={styles.timestamp}>{new Date(item.createdAt).toLocaleTimeString()}</Text>
            </View>
          </View>
        )}
        contentContainerStyle={styles.repliesList}
        ListEmptyComponent={
          <View style={styles.empty}>
            <Text style={styles.emptyText}>No replies yet</Text>
            <Text style={styles.emptySubtext}>Start the discussion!</Text>
          </View>
        }
      />

      {/* Reply input */}
      <KeyboardAvoidingView behavior={Platform.OS === "ios" ? "padding" : "height"}>
        <View style={styles.composer}>
          <TextInput
            style={styles.input}
            placeholder="Reply to thread..."
            placeholderTextColor="#7890b0"
            value={input}
            onChangeText={setInput}
            multiline
            maxLength={4000}
          />
          <Pressable onPress={handleReply} style={styles.sendBtn} disabled={loading || !input.trim()}>
            {loading ? (
              <ActivityIndicator size="small" color="#062020" />
            ) : (
              <Text style={[styles.sendBtnText, !input.trim() && styles.sendBtnDisabled]}>Reply</Text>
            )}
          </Pressable>
        </View>
      </KeyboardAvoidingView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1, backgroundColor: "#030a14" },
  header: { flexDirection: "row", alignItems: "center", padding: 12, backgroundColor: "#071327", borderBottomWidth: 1, borderBottomColor: "#0f2040" },
  backBtn: { paddingRight: 10 },
  backBtnText: { color: "#27c9a5", fontSize: 15, fontWeight: "600" },
  headerTitleArea: { flex: 1 },
  headerTitle: { color: "#d6f4ff", fontSize: 16, fontWeight: "700" },
  headerMeta: { color: "#7890b0", fontSize: 12, marginTop: 2 },
  parentMessage: { flexDirection: "row", padding: 16, backgroundColor: "#0b1a30", borderBottomWidth: 1, borderBottomColor: "#0f2040" },
  repliesList: { padding: 16 },
  reply: { flexDirection: "row", marginBottom: 16 },
  avatar: { width: 36, height: 36, borderRadius: 18, backgroundColor: "#1e3a5f", justifyContent: "center", alignItems: "center", marginRight: 10 },
  avatarText: { color: "#9ec5d5", fontSize: 14, fontWeight: "600" },
  messageContent: { flex: 1 },
  authorName: { color: "#9ec5d5", fontSize: 13, fontWeight: "600", marginBottom: 4 },
  messageText: { color: "#d6f4ff", fontSize: 15, lineHeight: 20 },
  timestamp: { color: "#7890b0", fontSize: 11, marginTop: 4 },
  empty: { alignItems: "center", paddingTop: 40 },
  emptyText: { color: "#9ec5d5", fontSize: 16, fontWeight: "600" },
  emptySubtext: { color: "#7890b0", fontSize: 14, marginTop: 8 },
  composer: { flexDirection: "row", alignItems: "flex-end", backgroundColor: "#071327", padding: 12, borderTopWidth: 1, borderTopColor: "#0f2040" },
  input: { flex: 1, backgroundColor: "#112544", color: "#d6f4ff", borderRadius: 20, paddingHorizontal: 16, paddingVertical: 10, fontSize: 15, maxHeight: 100, marginRight: 10 },
  sendBtn: { backgroundColor: "#27c9a5", borderRadius: 20, paddingHorizontal: 16, paddingVertical: 10 },
  sendBtnText: { color: "#062020", fontWeight: "700", fontSize: 15 },
  sendBtnDisabled: { opacity: 0.4 },
});
