/**
 * SearchScreen.tsx - Global search (messages, users, servers)
 */
import React, { useState, useCallback } from "react";
import {
  View, Text, FlatList, TextInput, Pressable, StyleSheet,
  SafeAreaView, ActivityIndicator, Keyboard
} from "react-native";
import { useRouter } from "expo-router";
import { store } from "../lib/store";
import { api } from "../lib/api";
import { debounce } from "lodash";

type SearchTab = "messages" | "users" | "servers";

interface SearchResult {
  id: string;
  type: SearchTab;
  title: string;
  subtitle: string;
  channelId?: string;
  serverId?: string;
}

export default function SearchScreen() {
  const router = useRouter();
  const [query, setQuery] = useState("");
  const [activeTab, setActiveTab] = useState<SearchTab>("messages");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [recent, setRecent] = useState<string[]>([]);

  // Debounced search
  const performSearch = useCallback(
    debounce(async (q: string, tab: SearchTab) => {
      if (!q.trim()) {
        setResults([]);
        return;
      }
      setLoading(true);
      try {
        let searchResults: SearchResult[] = [];
        
        switch (tab) {
          case "messages": {
            const data = await api.search.messages(q);
            searchResults = data.messages.map((m: any) => ({
              id: m.id,
              type: "messages" as SearchTab,
              title: m.content.slice(0, 100) || "(no content)",
              subtitle: `${m.authorUsername} in #${m.channelName || "channel"}`,
              channelId: m.channelId,
            }));
            break;
          }
          case "users": {
            const data = await api.search.users(q);
            searchResults = data.map((u: any) => ({
              id: u.id,
              type: "users" as SearchTab,
              title: u.displayName || u.username,
              subtitle: `@${u.username}`,
            }));
            break;
          }
          case "servers": {
            const data = await api.directory.search(q);
            searchResults = data.map((s: any) => ({
              id: s.id,
              type: "servers" as SearchTab,
              title: s.name,
              subtitle: `${s.memberCount || 0} members • ${s.category || "Community"}`,
              serverId: s.id,
            }));
            break;
          }
        }
        setResults(searchResults);
      } catch (e) {
        console.error("Search failed:", e);
      } finally {
        setLoading(false);
      }
    }, 300),
    []
  );

  function onSearch(text: string) {
    setQuery(text);
    performSearch(text, activeTab);
  }

  function onTabChange(tab: SearchTab) {
    setActiveTab(tab);
    if (query.trim()) {
      performSearch(query, tab);
    }
  }

  function handleSelect(result: SearchResult) {
    // Save to recent
    setRecent(prev => [result.title, ...prev.slice(0, 9)]);
    
    // Navigate
    if (result.type === "messages" && result.channelId) {
      store.selectChannel(result.channelId);
      router.push(`/channel/${result.channelId}`);
    } else if (result.type === "users") {
      router.push(`/user/${result.id}`);
    } else if (result.type === "servers" && result.serverId) {
      store.joinServer(result.serverId).then(() => {
        store.selectServer(result.serverId!);
        router.push(`/server/${result.serverId}`);
      });
    }
  }

  function renderResult({ item }: { item: SearchResult }) {
    return (
      <Pressable style={styles.resultRow} onPress={() => handleSelect(item)}>
        <View style={styles.resultIcon}>
          <Text style={styles.resultIconText}>
            {item.type === "messages" ? "💬" : item.type === "users" ? "👤" : "🏠"}
          </Text>
        </View>
        <View style={styles.resultInfo}>
          <Text style={styles.resultTitle} numberOfLines={2}>{item.title}</Text>
          <Text style={styles.resultSubtitle} numberOfLines={1}>{item.subtitle}</Text>
        </View>
      </Pressable>
    );
  }

  return (
    <SafeAreaView style={styles.container}>
      {/* Search header */}
      <View style={styles.header}>
        <Pressable onPress={() => router.back()} style={styles.cancelBtn}>
          <Text style={styles.cancelBtnText}>Cancel</Text>
        </Pressable>
        <View style={styles.searchBox}>
          <Text style={styles.searchIcon}>🔍</Text>
          <TextInput
            style={styles.searchInput}
            placeholder="Search Nexus"
            placeholderTextColor="#7890b0"
            value={query}
            onChangeText={onSearch}
            autoFocus
            returnKeyType="search"
            onSubmitEditing={() => Keyboard.dismiss()}
          />
          {query.length > 0 && (
            <Pressable onPress={() => onSearch("")} style={styles.clearBtn}>
              <Text style={styles.clearBtnText}>✕</Text>
            </Pressable>
          )}
        </View>
      </View>

      {/* Tab bar */}
      <View style={styles.tabBar}>
        {(["messages", "users", "servers"] as SearchTab[]).map(tab => (
          <Pressable
            key={tab}
            style={[styles.tab, activeTab === tab && styles.tabActive]}
            onPress={() => onTabChange(tab)}
          >
            <Text style={[styles.tabText, activeTab === tab && styles.tabTextActive]}>
              {tab.charAt(0).toUpperCase() + tab.slice(1)}
            </Text>
          </Pressable>
        ))}
      </View>

      {/* Results */}
      {loading ? (
        <View style={styles.center}>
          <ActivityIndicator size="large" color="#27c9a5" />
        </View>
      ) : results.length > 0 ? (
        <FlatList
          data={results}
          keyExtractor={item => `${item.type}-${item.id}`}
          renderItem={renderResult}
          contentContainerStyle={styles.resultsList}
          keyboardDismissMode="on-drag"
        />
      ) : query.length > 0 ? (
        <View style={styles.center}>
          <Text style={styles.emptyText}>No results found</Text>
          <Text style={styles.emptySubtext}>Try different keywords</Text>
        </View>
      ) : (
        <View style={styles.recentSection}>
          <Text style={styles.sectionTitle}>Recent Searches</Text>
          {recent.length === 0 ? (
            <Text style={styles.noRecent}>No recent searches</Text>
          ) : (
            recent.map((r, i) => (
              <Pressable key={i} style={styles.recentRow} onPress={() => onSearch(r)}>
                <Text style={styles.recentIcon}>🕐</Text>
                <Text style={styles.recentText}>{r}</Text>
              </Pressable>
            ))
          )}
        </View>
      )}
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1, backgroundColor: "#030a14" },
  header: { flexDirection: "row", alignItems: "center", padding: 12, borderBottomWidth: 1, borderBottomColor: "#0f2040" },
  cancelBtn: { paddingRight: 12 },
  cancelBtnText: { color: "#27c9a5", fontSize: 16 },
  searchBox: { flex: 1, flexDirection: "row", alignItems: "center", backgroundColor: "#0b1a30", borderRadius: 10, paddingHorizontal: 10, height: 40 },
  searchIcon: { fontSize: 16, marginRight: 8 },
  searchInput: { flex: 1, color: "#d6f4ff", fontSize: 16 },
  clearBtn: { padding: 4 },
  clearBtnText: { color: "#7890b0", fontSize: 16 },
  tabBar: { flexDirection: "row", borderBottomWidth: 1, borderBottomColor: "#0f2040" },
  tab: { flex: 1, paddingVertical: 12, alignItems: "center" },
  tabActive: { borderBottomWidth: 2, borderBottomColor: "#27c9a5" },
  tabText: { color: "#7890b0", fontSize: 14, fontWeight: "600" },
  tabTextActive: { color: "#27c9a5" },
  center: { flex: 1, justifyContent: "center", alignItems: "center" },
  emptyText: { color: "#9ec5d5", fontSize: 16, fontWeight: "600" },
  emptySubtext: { color: "#7890b0", fontSize: 14, marginTop: 8 },
  resultsList: { padding: 8 },
  resultRow: { flexDirection: "row", alignItems: "center", padding: 12, backgroundColor: "#0b1a30", borderRadius: 8, marginBottom: 6 },
  resultIcon: { width: 40, height: 40, borderRadius: 20, backgroundColor: "#112544", justifyContent: "center", alignItems: "center", marginRight: 12 },
  resultIconText: { fontSize: 20 },
  resultInfo: { flex: 1 },
  resultTitle: { color: "#d6f4ff", fontSize: 15, lineHeight: 20 },
  resultSubtitle: { color: "#7890b0", fontSize: 13, marginTop: 2 },
  recentSection: { padding: 16 },
  sectionTitle: { color: "#7890b0", fontSize: 13, fontWeight: "700", textTransform: "uppercase", letterSpacing: 0.5, marginBottom: 12 },
  noRecent: { color: "#7890b0", fontSize: 14, fontStyle: "italic" },
  recentRow: { flexDirection: "row", alignItems: "center", paddingVertical: 10 },
  recentIcon: { fontSize: 16, marginRight: 12, width: 24 },
  recentText: { color: "#d6f4ff", fontSize: 15 },
});
