import { useMemo, useState } from "react";
import { Pressable, SafeAreaView, ScrollView, Text, TextInput, View } from "react-native";

type Message = {
  id: string;
  author: string;
  content: string;
};

type ExperienceMode = "full" | "messaging";

type ExperienceProfile = {
  experience_mode: ExperienceMode;
  default_surface: string;
  enabled_modules: string[];
};

const API_BASE_DEFAULT = "http://localhost:8080/api/v1";

export default function HomeScreen() {
  const [apiBase, setApiBase] = useState(API_BASE_DEFAULT);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [token, setToken] = useState<string | null>(null);
  const [experienceMode, setExperienceMode] = useState<ExperienceMode>("full");
  const [status, setStatus] = useState<string>("Not authenticated");
  const [messages, setMessages] = useState<Message[]>([]);
  const [draft, setDraft] = useState("");

  const signedIn = useMemo(() => token !== null, [token]);

  function authHeaders(accessToken: string): HeadersInit {
    return {
      "Content-Type": "application/json",
      Authorization: `Bearer ${accessToken}`,
    };
  }

  async function login() {
    setStatus("Signing in...");
    const res = await fetch(`${apiBase}/auth/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ username, password }),
    });
    if (!res.ok) {
      setStatus(`Auth failed (${res.status})`);
      return;
    }
    const body = await res.json();
    if (body.access_token) {
      const accessToken = body.access_token as string;
      setToken(accessToken);
      setStatus("Authenticated");
      await loadExperienceProfile(accessToken);
    }
  }

  async function loadExperienceProfile(accessToken: string) {
    const res = await fetch(`${apiBase}/users/@me/experience-profile`, {
      headers: authHeaders(accessToken),
    });
    if (!res.ok) {
      setStatus(`Profile load failed (${res.status})`);
      return;
    }
    const body = (await res.json()) as ExperienceProfile;
    if (body.experience_mode === "full" || body.experience_mode === "messaging") {
      setExperienceMode(body.experience_mode);
    }
  }

  async function setMode(nextMode: ExperienceMode) {
    if (!token || nextMode === experienceMode) return;
    setStatus("Updating experience mode...");
    const res = await fetch(`${apiBase}/users/@me/experience-profile`, {
      method: "PUT",
      headers: authHeaders(token),
      body: JSON.stringify({ experience_mode: nextMode }),
    });
    if (!res.ok) {
      setStatus(`Mode update failed (${res.status})`);
      return;
    }
    const body = (await res.json()) as ExperienceProfile;
    if (body.experience_mode === "full" || body.experience_mode === "messaging") {
      setExperienceMode(body.experience_mode);
      setStatus(`Mode set to ${body.experience_mode}`);
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
          <Text style={{ color: signedIn ? "#8fffcd" : "#ff9aa9" }}>{status}</Text>
        </View>

        {signedIn ? (
          <View style={{ backgroundColor: "#0b1a30", borderRadius: 12, padding: 12, gap: 10 }}>
            <Text style={{ color: "#d6f4ff", fontWeight: "700" }}>Experience Mode</Text>
            <View style={{ flexDirection: "row", gap: 8 }}>
              <Pressable
                onPress={() => setMode("full")}
                style={{
                  backgroundColor: experienceMode === "full" ? "#5fd668" : "#112544",
                  borderRadius: 10,
                  paddingVertical: 10,
                  paddingHorizontal: 14,
                }}
              >
                <Text style={{ color: experienceMode === "full" ? "#102010" : "#d6f4ff", fontWeight: "700" }}>
                  Full
                </Text>
              </Pressable>
              <Pressable
                onPress={() => setMode("messaging")}
                style={{
                  backgroundColor: experienceMode === "messaging" ? "#5fd668" : "#112544",
                  borderRadius: 10,
                  paddingVertical: 10,
                  paddingHorizontal: 14,
                }}
              >
                <Text style={{ color: experienceMode === "messaging" ? "#102010" : "#d6f4ff", fontWeight: "700" }}>
                  Messaging
                </Text>
              </Pressable>
            </View>
          </View>
        ) : null}

        {signedIn && experienceMode === "full" ? (
          <View style={{ backgroundColor: "#0b1a30", borderRadius: 12, padding: 12, gap: 8 }}>
            <Text style={{ color: "#d6f4ff", fontWeight: "700" }}>Full Experience Preview</Text>
            <Text style={{ color: "#9ec5d5" }}>
              Server discovery, communities, plugins, collaboration boards, and media tools
              are enabled in this mode.
            </Text>
          </View>
        ) : null}

        <View style={{ backgroundColor: "#0b1a30", borderRadius: 12, padding: 12, gap: 10 }}>
          <Text style={{ color: "#d6f4ff", fontWeight: "700" }}>
            {experienceMode === "messaging" ? "Messaging Mode" : "Chat Prototype"}
          </Text>
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
