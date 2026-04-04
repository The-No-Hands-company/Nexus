/**
 * app/_layout.tsx - Root layout with reactive auth guard
 */
import React, { useEffect, useState, createContext, useContext } from "react";
import { View, ActivityIndicator, StyleSheet } from "react-native";
import { Stack } from "expo-router";
import { store } from "./lib/store";

export default function RootLayout() {
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const sub = store.addListener(() => {
      setLoading(false);
    });
    setLoading(store.session === null);
    return sub;
  }, []);

  if (loading) {
    return (
      <View style={styles.center}>
        <ActivityIndicator size="large" color="#27c9a5" />
      </View>
    );
  }

  return (
    <Stack
      screenOptions={{
        headerShown: false,
        contentStyle: { backgroundColor: "#030a14" },
        animation: "fade",
      }}
    >
      <Stack.Screen name="index" options={{ headerShown: false }} />
      <Stack.Screen name="(auth)/login" options={{ headerShown: false }} />
      <Stack.Screen name="(tabs)" options={{ headerShown: false }} />
      <Stack.Screen name="server/[id]" options={{ headerShown: false }} />
      <Stack.Screen name="channel/[id]" options={{ headerShown: false }} />
    </Stack>
  );
}

const styles = {
  center: { flex: 1, justifyContent: "center" as const, alignItems: "center" as const, backgroundColor: "#030a14" },
};
