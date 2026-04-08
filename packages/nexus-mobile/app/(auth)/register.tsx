import { useState } from "react";
import {
  View, Text, TextInput, Pressable, StyleSheet,
  KeyboardAvoidingView, Platform, ScrollView, ActivityIndicator, Alert,
} from "react-native";
import { useRouter, Link } from "expo-router";
import { store } from "../lib/store";
import { getServerUrlPlaceholder } from "../lib/api";

export default function RegisterScreen() {
  const router = useRouter();
  const [serverUrl, setServerUrl] = useState(store.serverUrl || "");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [email, setEmail] = useState("");
  const [loading, setLoading] = useState(false);

  async function submit() {
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
      await store.setServerUrl(serverUrl);
      await store.register(username.trim(), password, email.trim() || undefined);
      if (store.error) {
        Alert.alert("Error", store.error);
      } else {
        router.replace("/(tabs)");
      }
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
            placeholder={getServerUrlPlaceholder()}
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
  btn:       {
    marginTop: 20, backgroundColor: "#27c9a5", borderRadius: 10,
    paddingVertical: 14, alignItems: "center",
  },
  btnDisabled: { opacity: 0.6 },
  btnText:   { color: "#062020", fontWeight: "700", fontSize: 16 },
  footer:    { color: "#7890b0", textAlign: "center", marginTop: 20, fontSize: 14 },
  link:      { color: "#27c9a5" },
});