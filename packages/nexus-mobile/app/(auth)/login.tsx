/**
 * LoginScreen.tsx - Authentication (login + register)
 */
import React, { useState } from "react";
import {
  View, Text, TextInput, Pressable, StyleSheet,
  SafeAreaView, ScrollView, KeyboardAvoidingView,
  Platform, ActivityIndicator, Alert, Linking } from "react-native";
import { useRouter, Stack } from "expo-router";
import { store } from "../lib/store";
import { getServerUrlPlaceholder } from "../lib/api";

export default function LoginScreen() {
  const router = useRouter();
  const [serverUrl, setServerUrl] = useState(store.serverUrl || "");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);

  async function handleSubmit() {
    if (!serverUrl.trim()) {
      Alert.alert("Error", "Server URL is required");
      return;
    }
    if (!username.trim() || !password.trim()) {
      Alert.alert("Error", "Please fill in all required fields");
      return;
    }
    setLoading(true);
    try {
      await store.setServerUrl(serverUrl.trim());
      // One credential for the whole ecosystem, checked by Auth. This screen
      // no longer creates accounts: access is invite-only and needs an
      // operator to approve the request, which happens on the web.
      await store.login(username.trim(), password);
      if (store.error) {
        Alert.alert("Error", store.error);
        return;
      }
      router.replace("/(tabs)");
    } catch (e: unknown) {
      Alert.alert("Error", e instanceof Error ? e.message : "Authentication failed.");
    } finally {
      setLoading(false);
    }
  }

  return (
    <SafeAreaView style={styles.container}>
      <Stack.Screen options={{ title: "Nexus", headerShown: false }} />
      <KeyboardAvoidingView
        behavior={Platform.OS === "ios" ? "padding" : "height"}
        style={styles.flex}
      >
        <ScrollView contentContainerStyle={styles.scroll} keyboardShouldPersistTaps="handled">
          <View style={styles.logoArea}>
            <Text style={styles.logo}>N</Text>
            <Text style={styles.title}>Nexus</Text>
            <Text style={styles.subtitle}>Privacy-first communication</Text>
          </View>

          <View style={styles.card}>
            <TextInput
              style={styles.input}
              placeholder="Username"
              placeholderTextColor="#7890b0"
              value={username}
              onChangeText={setUsername}
              autoCapitalize="none"
              autoCorrect={false}
            />
            <TextInput
              style={styles.input}
              placeholder={getServerUrlPlaceholder()}
              placeholderTextColor="#7890b0"
              value={serverUrl}
              onChangeText={setServerUrl}
              autoCapitalize="none"
              autoCorrect={false}
              keyboardType="url"
            />
            <TextInput
              style={styles.input}
              placeholder="Password"
              placeholderTextColor="#7890b0"
              value={password}
              onChangeText={setPassword}
              secureTextEntry
            />

            <Pressable
              onPress={handleSubmit}
              style={[styles.button, loading && styles.buttonDisabled]}
              disabled={loading}
            >
              {loading ? (
                <ActivityIndicator color="#062020" />
              ) : (
                <Text style={styles.buttonText}>Sign In</Text>
              )}
            </Pressable>

            {/*
              Accounts are not created here. Access is invite-only: a request
              has to be approved by an operator before an account exists, and
              that flow lives on the web. A half-copy of it in the app would be
              a second place to keep correct.
            */}
            <Pressable onPress={() => Linking.openURL(store.requestAccessUrl())}>
              <Text style={styles.note}>
                No account? Request access — opens in your browser.
              </Text>
            </Pressable>

            <Text style={styles.note}>
              One account for every Nexus app. No phone, no ID, no surveillance.
            </Text>
          </View>
        </ScrollView>
      </KeyboardAvoidingView>
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  flex: { flex: 1 },
  container: { flex: 1, backgroundColor: "#030a14" },
  scroll: { flexGrow: 1, justifyContent: "center", padding: 20 },
  logoArea: { alignItems: "center", marginBottom: 32 },
  logo: {
    fontSize: 56, fontWeight: "900", color: "#27c9a5",
    backgroundColor: "#0b1a30", width: 80, height: 80,
    textAlign: "center", lineHeight: 80, borderRadius: 20,
    overflow: "hidden", marginBottom: 12,
  },
  title: { fontSize: 32, fontWeight: "800", color: "#d6f4ff" },
  subtitle: { fontSize: 14, color: "#9ec5d5", marginTop: 4 },
  card: { backgroundColor: "#0b1a30", borderRadius: 16, padding: 20, gap: 12 },
  tabs: { flexDirection: "row", marginBottom: 8 },
  tab: { flex: 1, paddingVertical: 10, alignItems: "center", borderBottomWidth: 2, borderBottomColor: "transparent" },
  tabActive: { borderBottomColor: "#27c9a5" },
  tabText: { color: "#9ec5d5", fontWeight: "600", fontSize: 15 },
  tabTextActive: { color: "#27c9a5" },
  input: {
    backgroundColor: "#112544", color: "#d6f4ff",
    borderRadius: 10, padding: 14, fontSize: 16,
    borderWidth: 1, borderColor: "#1e3a5f",
  },
  button: {
    backgroundColor: "#27c9a5", borderRadius: 10, padding: 14,
    alignItems: "center", marginTop: 4,
  },
  buttonDisabled: { opacity: 0.6 },
  buttonText: { color: "#062020", fontWeight: "700", fontSize: 16 },
  note: { fontSize: 12, color: "#9ec5d5", textAlign: "center", marginTop: 4 },
});
