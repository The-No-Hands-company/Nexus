import { useState, useEffect } from "react";
import {
  View, Text, TextInput, Pressable, StyleSheet,
  KeyboardAvoidingView, Platform, ScrollView, ActivityIndicator,
} from "react-native";
import { useRouter, Link } from "expo-router";
import { store } from "../lib/store";

export default function RegisterScreen() {
  const router = useRouter();
  const [serverUrl, setServerUrl] = useState(store.serverUrl || "http://localhost:8080");
  const [username, setUsername]   = useState("");
  const [password, setPassword]   = useState("");
  const [email, setEmail]         = useState("");
  const [error, setError]         = useState<string | null>(null);
  const [loading, setLoading]     = useState(false);
  

  async function submit() {
    if (!username.trim() || !password) return;
    setError(null);
    setLoading(true);
    try {
      const base = serverUrl.replace(/\/$/, "");
      await store.setServerUrl(base);
      await store.register(username.trim(), password, email.trim() || undefined);
      router.replace("/(tabs)");
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setLoading(false);
    }
  }

  return (
    <KeyboardAvoidingView
      style={s.root}
      behavior={Platform.OS === "ios" ? "padding" : "height"}
    >
      <ScrollView
        contentContainerStyle={s.scroll}
        keyboardShouldPersistTaps="handled"
      >
        <View style={s.logoRow}>
          <View style={s.logoBox}>
            <Text style={s.logoText}>NX</Text>
          </View>
          <Text style={s.title}>Create account</Text>
          <Text style={s.subtitle}>No ID required. No phone number. Ever.</Text>
        </View>

        <View style={s.card}>
          <Label>Server URL</Label>
          <TextInput
            style={s.input}
            value={serverUrl}
            onChangeText={setServerUrl}
            autoCapitalize="none"
            autoCorrect={false}
            keyboardType="url"
            placeholder="http://localhost:8080"
            placeholderTextColor="#556680"
          />

          <Label>Username</Label>
          <TextInput
            style={s.input}
            value={username}
            onChangeText={setUsername}
            autoCapitalize="none"
            autoCorrect={false}
            placeholder="your-username"
            placeholderTextColor="#556680"
            maxLength={32}
          />

          <Label>Password</Label>
          <TextInput
            style={s.input}
            value={password}
            onChangeText={setPassword}
            secureTextEntry
            placeholder="Min. 8 characters"
            placeholderTextColor="#556680"
          />

          <Label>Email (optional — for password reset only)</Label>
          <TextInput
            style={s.input}
            value={email}
            onChangeText={setEmail}
            autoCapitalize="none"
            keyboardType="email-address"
            placeholder="optional"
            placeholderTextColor="#556680"
          />

          {error && (
            <View style={s.errorBox}>
              <Text style={s.errorText}>{error}</Text>
            </View>
          )}

          <Pressable
            style={[s.btn, loading && s.btnDisabled]}
            onPress={submit}
            disabled={loading}
          >
            {loading
              ? <ActivityIndicator color="#062020" />
              : <Text style={s.btnText}>Create Account</Text>}
          </Pressable>
        </View>

        <Text style={s.footer}>
          Have an account?{" "}
          <Link href="/(auth)/login" style={s.link}>Sign in</Link>
        </Text>
      </ScrollView>
    </KeyboardAvoidingView>
  );
}

function Label({ children }: { children: string }) {
  return <Text style={labelStyle}>{children}</Text>;
}
const labelStyle = {
  fontSize: 11, fontWeight: "600" as const, color: "#7890b0",
  textTransform: "uppercase" as const, letterSpacing: 0.8, marginBottom: 4, marginTop: 12,
};

const s = StyleSheet.create({
  root:      { flex: 1, backgroundColor: "#030a14" },
  scroll:    { flexGrow: 1, justifyContent: "center", padding: 20 },
  logoRow:   { alignItems: "center", marginBottom: 28 },
  logoBox:   {
    width: 52, height: 52, borderRadius: 14,
    backgroundColor: "rgba(39,201,165,0.15)",
    alignItems: "center", justifyContent: "center", marginBottom: 12,
  },
  logoText:  { color: "#27c9a5", fontWeight: "700", fontSize: 18 },
  title:     { color: "#d6f4ff", fontSize: 22, fontWeight: "600" },
  subtitle:  { color: "#7890b0", fontSize: 14, marginTop: 4, textAlign: "center" },
  card:      {
    backgroundColor: "#0b1a30", borderRadius: 16,
    padding: 20, borderWidth: 1, borderColor: "rgba(255,255,255,0.06)",
  },
  input:     {
    backgroundColor: "#112544", color: "#d6f4ff", borderRadius: 10,
    paddingHorizontal: 14, paddingVertical: 12, fontSize: 15,
    borderWidth: 1, borderColor: "rgba(255,255,255,0.08)",
  },
  errorBox:  {
    marginTop: 12, backgroundColor: "rgba(239,68,68,0.1)",
    borderRadius: 8, padding: 10, borderWidth: 1, borderColor: "rgba(239,68,68,0.3)",
  },
  errorText: { color: "#f87171", fontSize: 13 },
  btn:       {
    marginTop: 20, backgroundColor: "#27c9a5", borderRadius: 10,
    paddingVertical: 14, alignItems: "center",
  },
  btnDisabled: { opacity: 0.6 },
  btnText:   { color: "#062020", fontWeight: "700", fontSize: 16 },
  footer:    { color: "#7890b0", textAlign: "center", marginTop: 20, fontSize: 14 },
  link:      { color: "#27c9a5" },
});