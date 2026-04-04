import { useEffect } from "react";
import {
  View, Text, FlatList, Pressable, StyleSheet, ActivityIndicator,
} from "react-native";
import { useRouter } from "expo-router";
import { useStore } from "../../src/store";

export default function HomeScreen() {
  const { dmChannels, loadDms, session, setActiveChannel, loadMessages, unread } = useStore();
  const router = useRouter();

  useEffect(() => {
    if (session) loadDms();
  }, [session?.accessToken]);

  const open = (id: string) => {
    setActiveChannel(id);
    loadMessages(id);
    router.push(`/(app)/channel/${id}`);
  };

  const getDmName = (dm: (typeof dmChannels)[0]) => {
    if (dm.name) return dm.name;
    return (
      dm.recipients
        ?.filter((r) => r.id !== session?.userId)
        .map((r) => r.username)
        .join(", ") ?? "Direct Message"
    );
  };

  const getInitials = (name: string) => name.slice(0, 2).toUpperCase();

  return (
    <View style={s.root}>
      {dmChannels.length === 0 ? (
        <View style={s.empty}>
          <Text style={s.emptyEmoji}>💬</Text>
          <Text style={s.emptyTitle}>No messages yet</Text>
          <Text style={s.emptyBody}>
            Start a conversation by searching for a user on your server.
          </Text>
        </View>
      ) : (
        <FlatList
          data={dmChannels}
          keyExtractor={(dm) => dm.id}
          contentContainerStyle={s.list}
          renderItem={({ item: dm }) => {
            const name = getDmName(dm);
            const hasUnread = !!unread[dm.id];
            return (
              <Pressable
                style={({ pressed }) => [s.row, pressed && s.rowPressed]}
                onPress={() => open(dm.id)}
              >
                <View style={s.avatar}>
                  <Text style={s.avatarText}>{getInitials(name)}</Text>
                </View>
                <View style={s.rowBody}>
                  <Text style={[s.name, hasUnread && s.nameUnread]}>{name}</Text>
                </View>
                {hasUnread && <View style={s.dot} />}
              </Pressable>
            );
          }}
        />
      )}
    </View>
  );
}

const s = StyleSheet.create({
  root: { flex: 1, backgroundColor: "#030a14" },
  list: { padding: 12, gap: 2 },
  row: {
    flexDirection: "row", alignItems: "center", gap: 12,
    paddingVertical: 10, paddingHorizontal: 12, borderRadius: 10,
  },
  rowPressed: { backgroundColor: "#0b1a30" },
  avatar: {
    width: 40, height: 40, borderRadius: 20,
    backgroundColor: "rgba(39,201,165,0.2)",
    alignItems: "center", justifyContent: "center",
  },
  avatarText: { color: "#27c9a5", fontWeight: "700", fontSize: 13 },
  rowBody: { flex: 1 },
  name: { color: "#9ec5d5", fontSize: 15, fontWeight: "500" },
  nameUnread: { color: "#d6f4ff", fontWeight: "700" },
  dot: {
    width: 9, height: 9, borderRadius: 5,
    backgroundColor: "#d6f4ff",
  },
  empty: { flex: 1, alignItems: "center", justifyContent: "center", padding: 32 },
  emptyEmoji: { fontSize: 48, marginBottom: 12 },
  emptyTitle: { color: "#d6f4ff", fontSize: 18, fontWeight: "600", marginBottom: 8 },
  emptyBody: { color: "#7890b0", fontSize: 14, textAlign: "center", lineHeight: 20 },
});
