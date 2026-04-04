import { useEffect } from "react";
import {
  View, Text, FlatList, Pressable, StyleSheet,
  RefreshControl, ActivityIndicator,
} from "react-native";
import { useRouter } from "expo-router";
import { useStore } from "../../src/store";

export default function ServersScreen() {
  const { servers, loadServers, session, setActiveServer, loadChannels } = useStore();
  const router = useRouter();
  const [refreshing, setRefreshing] = useState(false);

  useEffect(() => {
    if (session) loadServers();
  }, [session?.accessToken]);

  const refresh = async () => {
    setRefreshing(true);
    await loadServers();
    setRefreshing(false);
  };

  const open = (id: string) => {
    setActiveServer(id);
    loadChannels(id);
    router.push(`/(app)/server/${id}`);
  };

  return (
    <View style={s.root}>
      {servers.length === 0 ? (
        <View style={s.empty}>
          <Text style={s.emptyEmoji}>🏠</Text>
          <Text style={s.emptyTitle}>No servers yet</Text>
          <Text style={s.emptyBody}>Join or create a server on the web client to get started.</Text>
        </View>
      ) : (
        <FlatList
          data={servers}
          keyExtractor={(srv) => srv.id}
          contentContainerStyle={s.list}
          refreshControl={
            <RefreshControl
              refreshing={refreshing}
              onRefresh={refresh}
              tintColor="#27c9a5"
            />
          }
          renderItem={({ item: srv }) => (
            <Pressable
              style={({ pressed }) => [s.row, pressed && s.rowPressed]}
              onPress={() => open(srv.id)}
            >
              <View style={s.icon}>
                {srv.icon ? (
                  // In a real app, use expo-image for cached images
                  <Text style={s.iconText}>{srv.name.slice(0, 2).toUpperCase()}</Text>
                ) : (
                  <Text style={s.iconText}>{srv.name.slice(0, 2).toUpperCase()}</Text>
                )}
              </View>
              <View style={s.rowBody}>
                <Text style={s.name}>{srv.name}</Text>
                {srv.description ? (
                  <Text style={s.desc} numberOfLines={1}>{srv.description}</Text>
                ) : null}
              </View>
              <Text style={s.chevron}>›</Text>
            </Pressable>
          )}
        />
      )}
    </View>
  );
}

// useState needs to be imported
import { useState } from "react";

const s = StyleSheet.create({
  root: { flex: 1, backgroundColor: "#030a14" },
  list: { padding: 12, gap: 4 },
  row: {
    flexDirection: "row", alignItems: "center", gap: 12,
    paddingVertical: 12, paddingHorizontal: 14,
    backgroundColor: "#0b1a30", borderRadius: 12,
    borderWidth: 1, borderColor: "rgba(255,255,255,0.04)",
  },
  rowPressed: { backgroundColor: "#112544" },
  icon: {
    width: 44, height: 44, borderRadius: 14,
    backgroundColor: "rgba(39,201,165,0.15)",
    alignItems: "center", justifyContent: "center",
  },
  iconText: { color: "#27c9a5", fontWeight: "700", fontSize: 14 },
  rowBody: { flex: 1 },
  name: { color: "#d6f4ff", fontSize: 16, fontWeight: "600" },
  desc: { color: "#7890b0", fontSize: 13, marginTop: 2 },
  chevron: { color: "#556680", fontSize: 22, marginRight: -2 },
  empty: { flex: 1, alignItems: "center", justifyContent: "center", padding: 32 },
  emptyEmoji: { fontSize: 48, marginBottom: 12 },
  emptyTitle: { color: "#d6f4ff", fontSize: 18, fontWeight: "600", marginBottom: 8 },
  emptyBody: { color: "#7890b0", fontSize: 14, textAlign: "center", lineHeight: 20 },
});
