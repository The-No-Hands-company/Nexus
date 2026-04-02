import { useMemo, useState } from "react";
import { Pressable, SafeAreaView, ScrollView, Text, TextInput, View } from "react-native";

type Message = {
  id: string;
  author: string;
  content: string;
};

const API_BASE_DEFAULT = "http://localhost:8080/api/v1";

export default function HomeScreen() {
  const [apiBase, setApiBase] = useState(API_BASE_DEFAULT);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [token, setToken] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [draft, setDraft] = useState("");

  const signedIn = useMemo(() => token !== null, [token]);

  async function login() {
    const res = await fetch(`${apiBase}/auth/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ username, password }),
    });
    if (!res.ok) {
      return;
    }
    const body = await res.json();
    if (body.access_token) {
      setToken(body.access_token);
    }
  }

  function appendLocalMessage() {
    if (!draft.trim()) return;
    setMessages((prev) => [
      { id: String(Date.now()), author: username || "me", content: draft.trim() },
      ...prev,
    ]);
    setDraft("");
  }

  return (
    <SafeAreaView style={{ flex: 1, backgroundColor: "#030a14" }}>
      <ScrollView contentContainerStyle={{ padding: 16, gap: 12 }}>
        <Text style={{ color: "#d6f4ff", fontSize: 24, fontWeight: "700" }}>Nexus Mobile</Text>
        <Text style={{ color: "#9ec5d5" }}>Bootstrap app for mobile parity roadmap.</Text>

        <View style={{ backgroundColor: "#0b1a30", borderRadius: 12, padding: 12, gap: 10 }}>
          <Text style={{ color: "#d6f4ff" }}>API Base</Text>
          <TextInput
            value={apiBase}
            onChangeText={setApiBase}
            autoCapitalize="none"
            style={{ backgroundColor: "#112544", color: "#d6f4ff", borderRadius: 10, padding: 10 }}
          />
          <TextInput
            placeholder="Username"
            placeholderTextColor="#7890b0"
            value={username}
            onChangeText={setUsername}
            autoCapitalize="none"
            style={{ backgroundColor: "#112544", color: "#d6f4ff", borderRadius: 10, padding: 10 }}
          />
          <TextInput
            placeholder="Password"
            placeholderTextColor="#7890b0"
            value={password}
            onChangeText={setPassword}
            secureTextEntry
            style={{ backgroundColor: "#112544", color: "#d6f4ff", borderRadius: 10, padding: 10 }}
          />
          <Pressable onPress={login} style={{ backgroundColor: "#27c9a5", borderRadius: 10, padding: 12 }}>
            <Text style={{ color: "#062020", fontWeight: "700", textAlign: "center" }}>Sign In</Text>
          </Pressable>
          <Text style={{ color: signedIn ? "#8fffcd" : "#ff9aa9" }}>
            {signedIn ? "Authenticated" : "Not authenticated"}
          </Text>
        </View>

        <View style={{ backgroundColor: "#0b1a30", borderRadius: 12, padding: 12, gap: 10 }}>
          <Text style={{ color: "#d6f4ff", fontWeight: "700" }}>Chat Prototype</Text>
          <View style={{ flexDirection: "row", gap: 8 }}>
            <TextInput
              value={draft}
              onChangeText={setDraft}
              placeholder="Type a message"
              placeholderTextColor="#7890b0"
              style={{ flex: 1, backgroundColor: "#112544", color: "#d6f4ff", borderRadius: 10, padding: 10 }}
            />
            <Pressable onPress={appendLocalMessage} style={{ backgroundColor: "#5fd668", borderRadius: 10, paddingHorizontal: 14, justifyContent: "center" }}>
              <Text style={{ color: "#102010", fontWeight: "700" }}>Send</Text>
            </Pressable>
          </View>

          {messages.map((m) => (
            <View key={m.id} style={{ backgroundColor: "#112544", borderRadius: 10, padding: 10 }}>
              <Text style={{ color: "#9ec5d5", fontSize: 12 }}>{m.author}</Text>
              <Text style={{ color: "#d6f4ff" }}>{m.content}</Text>
            </View>
          ))}
        </View>
      </ScrollView>
    </SafeAreaView>
  );
}
