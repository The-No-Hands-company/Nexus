/**
 * (tabs)/profile.tsx - User profile & settings
 */
import React from "react";
import {
  View, Text, Pressable, StyleSheet,
  SafeAreaView, ScrollView, Alert,
} from "react-native";
import { useRouter } from "expo-router";
import { store } from "../lib/store";

export default function ProfileScreen() {
  const router = useRouter();
  const user = store.session?.user;

  function handleLogout() {
    Alert.alert("Sign Out", "Are you sure you want to sign out?", [
      { text: "Cancel", style: "cancel" },
      { text: "Sign Out", style: "destructive", onPress: () => {
        store.logout();
        router.replace("/(auth)/login");
      }},
    ]);
  }

  if (!user) {
    return (
      <SafeAreaView style={styles.container}>
        <Text style={styles.errorText}>Not signed in</Text>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView style={styles.container}>
      <ScrollView contentContainerStyle={styles.content}>
        <View style={styles.avatarSection}>
          <View style={styles.avatar}>
            <Text style={styles.avatarText}>{user.username[0]?.toUpperCase()}</Text>
          </View>
          <Text style={styles.displayName}>{user.displayName ?? user.username}</Text>
          <Text style={styles.username}>@{user.username}</Text>
        </View>

        <View style={styles.section}>
          <Text style={styles.sectionTitle}>Account</Text>
          <Pressable style={styles.row}>
            <Text style={styles.rowLabel}>Email</Text>
            <Text style={styles.rowValue}>{user.email ?? "Not set"}</Text>
          </Pressable>
          <Pressable style={styles.row}>
            <Text style={styles.rowLabel}>Member since</Text>
            <Text style={styles.rowValue}>
              {user.createdAt ? new Date(user.createdAt).toLocaleDateString() : "N/A"}
            </Text>
          </Pressable>
        </View>

        <View style={styles.section}>
          <Text style={styles.sectionTitle}>Preferences</Text>
          <Pressable style={styles.row}>
            <Text style={styles.rowLabel}>Experience Mode</Text>
            <Text style={styles.rowValue}>{store.experienceMode}</Text>
          </Pressable>
        </View>

        <View style={styles.section}>
          <Text style={styles.sectionTitle}>Servers</Text>
          <Pressable style={styles.row}>
            <Text style={styles.rowLabel}>My Servers</Text>
            <Text style={styles.rowValue}>{store.servers.length}</Text>
          </Pressable>
        </View>

        <View style={styles.section}>
          <Pressable onPress={handleLogout} style={styles.logoutBtn}>
            <Text style={styles.logoutBtnText}>Sign Out</Text>
          </Pressable>
        </View>
      </ScrollView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1, backgroundColor: "#030a14" },
  content: { padding: 16 },
  avatarSection: { alignItems: "center", paddingVertical: 32 },
  avatar: { width: 80, height: 80, borderRadius: 40, backgroundColor: "#0b1a30", justifyContent: "center", alignItems: "center", marginBottom: 12 },
  avatarText: { color: "#27c9a5", fontSize: 32, fontWeight: "800" },
  displayName: { color: "#d6f4ff", fontSize: 22, fontWeight: "700" },
  username: { color: "#9ec5d5", fontSize: 15, marginTop: 4 },
  section: { backgroundColor: "#0b1a30", borderRadius: 12, padding: 8, marginBottom: 16 },
  sectionTitle: { color: "#9ec5d5", fontSize: 13, fontWeight: "600", paddingHorizontal: 8, paddingVertical: 8, textTransform: "uppercase" },
  row: { flexDirection: "row", justifyContent: "space-between", alignItems: "center", padding: 12, borderBottomWidth: 1, borderBottomColor: "#0f2040" },
  rowLabel: { color: "#d6f4ff", fontSize: 15 },
  rowValue: { color: "#7890b0", fontSize: 14 },
  logoutBtn: { backgroundColor: "#ed4245", borderRadius: 10, padding: 14, alignItems: "center" },
  logoutBtnText: { color: "#fff", fontWeight: "700", fontSize: 16 },
  errorText: { color: "#ed4245", textAlign: "center", marginTop: 40 },
});
