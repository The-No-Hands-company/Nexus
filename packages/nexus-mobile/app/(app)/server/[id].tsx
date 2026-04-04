import { useEffect } from "react";
import {
  View, Text, FlatList, Pressable, StyleSheet, SectionList,
} from "react-native";
import { useRouter, useLocalSearchParams, Stack } from "expo-router";
import { useStore } from "../../src/store";

export default function ServerScreen() {
  const { id } = useLocalSearchParams<{ id: string }>();
  const { channels, servers, loadChannels, setActiveChannel, loadMessages, unread, session } = useStore();
  const router = useRouter();

  const server = servers.find((s) => s.id === id);

  useEffect(() => {
    if (id && session) loadChannels(id);
  }, [id, session?.accessToken]);

  const open = (channelId: string) => {
    setActiveChannel(channelId);
    loadMessages(channelId);
    router.push(`/(app)/channel/${channelId}`);
  };

  const textChannels = channels.filter(
    (c) => c.kind === "text" || c.kind === "announcement"
  );
  const voiceChannels = channels.filter((c) => c.kind === "voice");

  const sections = [
    ...(textChannels.length > 0 ? [{ title: "Channels", data: textChannels }] : []),
    ...(voiceChannels.length > 0 ? [{ title: "Voice", data: voiceChannels }] : []),
  ];

  return (
    <View style={s.root}>
      <Stack.Screen options={{ title: server?.name ?? "Server" }} />

      {channels.length === 0 ? (
        <View style={s.empty}>
          <Text style={s.emptyText}>No channels yet</Text>
        </View>
      ) : (
        <SectionList
          sections={sections}
          keyExtractor={(ch) => ch.id}
          contentContainerStyle={s.list}
          stickySectionHeadersEnabled={false}
          renderSectionHeader={({ section: { title } }) => (
            <View style={s.sectionHeader}>
              <Text style={s.sectionTitle}>{title.toUpperCase()}</Text>
            </View>
          )}
          renderItem={({ item: ch }) => {
            const isVoice = ch.kind === "voice";
            const hasUnread = !!unread[ch.id];
            return (
              <Pressable
                style={({ pressed }) => [s.row, pressed && s.rowPressed]}
                onPress={() => !isVoice && open(ch.id)}
                disabled={isVoice}
              >
                <Text style={[s.prefix, isVoice && s.prefixVoice]}>
                  {isVoice ? "🔊" : "#"}
                </Text>
                <Text
                  style={[
                    s.name,
                    hasUnread && s.nameUnread,
                    isVoice && s.nameVoice,
                  ]}
                >
                  {ch.name}
                </Text>
                {ch.is_e2ee && (
                  <View style={s.e2eeBadge}>
                    <Text style={s.e2eeText}>E2E</Text>
                  </View>
                )}
                {hasUnread && !isVoice && <View style={s.dot} />}
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
  list: { paddingVertical: 8 },
  sectionHeader: {
    paddingHorizontal: 16, paddingTop: 16, paddingBottom: 4,
  },
  sectionTitle: {
    fontSize: 10, fontWeight: "700", color: "#556680",
    letterSpacing: 1.2,
  },
  row: {
    flexDirection: "row", alignItems: "center", gap: 8,
    paddingVertical: 10, paddingHorizontal: 16,
  },
  rowPressed: { backgroundColor: "rgba(255,255,255,0.04)" },
  prefix: { color: "#556680", fontSize: 15, width: 18, textAlign: "center" },
  prefixVoice: { fontSize: 13 },
  name: { flex: 1, color: "#7890b0", fontSize: 15 },
  nameUnread: { color: "#d6f4ff", fontWeight: "600" },
  nameVoice: { color: "#556680" },
  e2eeBadge: {
    backgroundColor: "rgba(74,222,128,0.15)",
    borderRadius: 4, paddingHorizontal: 5, paddingVertical: 1,
  },
  e2eeText: { color: "#4ade80", fontSize: 9, fontWeight: "700" },
  dot: {
    width: 8, height: 8, borderRadius: 4,
    backgroundColor: "#d6f4ff",
  },
  empty: { flex: 1, alignItems: "center", justifyContent: "center" },
  emptyText: { color: "#556680", fontSize: 15 },
});
