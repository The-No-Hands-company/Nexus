import { View, Text, Pressable, StyleSheet, ScrollView, Alert } from "react-native";
import { useStore } from "../../src/store";

export default function SettingsScreen() {
  const { session, setSession, gatewayStatus } = useStore();

  const statusColor =
    gatewayStatus === "online" ? "#4ade80"
    : gatewayStatus === "connecting" ? "#facc15"
    : "#f87171";

  const statusLabel =
    gatewayStatus === "online" ? "Connected"
    : gatewayStatus === "connecting" ? "Connecting…"
    : "Disconnected";

  function confirmLogout() {
    Alert.alert("Sign out", "Are you sure you want to sign out?", [
      { text: "Cancel", style: "cancel" },
      {
        text: "Sign out",
        style: "destructive",
        onPress: () => setSession(null),
      },
    ]);
  }

  return (
    <ScrollView style={s.root} contentContainerStyle={s.content}>
      {/* Profile card */}
      <View style={s.profileCard}>
        <View style={s.avatar}>
          <Text style={s.avatarText}>
            {session?.username?.[0]?.toUpperCase() ?? "?"}
          </Text>
        </View>
        <View>
          <Text style={s.username}>{session?.username}</Text>
          <Text style={s.serverUrl}>{session?.serverUrl}</Text>
        </View>
      </View>

      {/* Connection section */}
      <SectionHeader>Connection</SectionHeader>
      <View style={s.card}>
        <Row label="Gateway status">
          <View style={[s.statusDot, { backgroundColor: statusColor }]} />
          <Text style={[s.statusLabel, { color: statusColor }]}>{statusLabel}</Text>
        </Row>
        <Divider />
        <Row label="Server">{session?.serverUrl ?? "—"}</Row>
      </View>

      {/* Account section */}
      <SectionHeader>Account</SectionHeader>
      <View style={s.card}>
        <Row label="Username">{session?.username ?? "—"}</Row>
      </View>

      {/* Danger zone */}
      <SectionHeader>Danger zone</SectionHeader>
      <Pressable
        style={({ pressed }) => [s.logoutBtn, pressed && s.logoutBtnPressed]}
        onPress={confirmLogout}
      >
        <Text style={s.logoutText}>Sign out</Text>
      </Pressable>

      <Text style={s.version}>Nexus Mobile · v0.1.0</Text>
    </ScrollView>
  );
}

// ── Sub-components ────────────────────────────────────────────────────────────

function SectionHeader({ children }: { children: string }) {
  return <Text style={sh.text}>{children.toUpperCase()}</Text>;
}
const sh = StyleSheet.create({
  text: {
    fontSize: 11, fontWeight: "700", color: "#556680",
    letterSpacing: 1.2, paddingHorizontal: 16, marginTop: 24, marginBottom: 6,
  },
});

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <View style={r.row}>
      <Text style={r.label}>{label}</Text>
      {typeof children === "string" ? (
        <Text style={r.value} numberOfLines={1}>{children}</Text>
      ) : (
        <View style={r.valueRow}>{children}</View>
      )}
    </View>
  );
}
const r = StyleSheet.create({
  row: {
    flexDirection: "row", alignItems: "center", justifyContent: "space-between",
    paddingHorizontal: 16, paddingVertical: 13,
  },
  label: { color: "#9ec5d5", fontSize: 15 },
  value: { color: "#556680", fontSize: 14, maxWidth: "55%", textAlign: "right" },
  valueRow: { flexDirection: "row", alignItems: "center", gap: 6 },
});

function Divider() {
  return <View style={{ height: 1, backgroundColor: "rgba(255,255,255,0.04)", marginLeft: 16 }} />;
}

const s = StyleSheet.create({
  root: { flex: 1, backgroundColor: "#030a14" },
  content: { paddingBottom: 40 },
  profileCard: {
    flexDirection: "row", alignItems: "center", gap: 14,
    margin: 16, padding: 16,
    backgroundColor: "#0b1a30", borderRadius: 14,
    borderWidth: 1, borderColor: "rgba(255,255,255,0.06)",
  },
  avatar: {
    width: 52, height: 52, borderRadius: 26,
    backgroundColor: "rgba(39,201,165,0.2)",
    alignItems: "center", justifyContent: "center",
  },
  avatarText: { color: "#27c9a5", fontWeight: "700", fontSize: 20 },
  username: { color: "#d6f4ff", fontSize: 17, fontWeight: "700" },
  serverUrl: { color: "#556680", fontSize: 12, marginTop: 2 },
  card: {
    marginHorizontal: 16,
    backgroundColor: "#0b1a30", borderRadius: 12,
    borderWidth: 1, borderColor: "rgba(255,255,255,0.06)",
    overflow: "hidden",
  },
  statusDot: { width: 8, height: 8, borderRadius: 4 },
  statusLabel: { fontSize: 14, fontWeight: "600" },
  logoutBtn: {
    marginHorizontal: 16, marginTop: 4,
    backgroundColor: "rgba(239,68,68,0.1)", borderRadius: 12,
    paddingVertical: 15, alignItems: "center",
    borderWidth: 1, borderColor: "rgba(239,68,68,0.3)",
  },
  logoutBtnPressed: { backgroundColor: "rgba(239,68,68,0.2)" },
  logoutText: { color: "#f87171", fontWeight: "700", fontSize: 15 },
  version: {
    color: "#334155", fontSize: 12, textAlign: "center", marginTop: 32,
  },
});
