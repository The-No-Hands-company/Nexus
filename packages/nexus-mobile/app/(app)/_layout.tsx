import { Tabs } from "expo-router";
import { View, Text, StyleSheet } from "react-native";

export default function AppLayout() {
  return (
    <Tabs
      screenOptions={{
        headerStyle: { backgroundColor: "#071327" },
        headerTintColor: "#d6f4ff",
        headerTitleStyle: { fontWeight: "600" },
        headerShadowVisible: false,
        tabBarStyle: {
          backgroundColor: "#071327",
          borderTopColor: "rgba(255,255,255,0.06)",
          height: 56,
          paddingBottom: 6,
        },
        tabBarActiveTintColor: "#27c9a5",
        tabBarInactiveTintColor: "#556680",
        tabBarLabelStyle: { fontSize: 11, fontWeight: "600" },
      }}
    >
      <Tabs.Screen
        name="index"
        options={{
          title: "Direct Messages",
          tabBarLabel: "DMs",
          tabBarIcon: ({ color }) => <TabIcon label="💬" color={color} />,
        }}
      />
      <Tabs.Screen
        name="servers"
        options={{
          title: "Servers",
          tabBarIcon: ({ color }) => <TabIcon label="🏠" color={color} />,
        }}
      />
      <Tabs.Screen
        name="settings"
        options={{
          title: "Settings",
          tabBarIcon: ({ color }) => <TabIcon label="⚙️" color={color} />,
        }}
      />
      {/* These screens are pushed onto the stack, not tabs */}
      <Tabs.Screen name="server/[id]" options={{ href: null }} />
      <Tabs.Screen name="channel/[id]" options={{ href: null }} />
    </Tabs>
  );
}

function TabIcon({ label, color }: { label: string; color: string }) {
  return (
    <View style={tabIconStyle.wrap}>
      <Text style={[tabIconStyle.emoji, { opacity: color === "#27c9a5" ? 1 : 0.5 }]}>
        {label}
      </Text>
    </View>
  );
}

const tabIconStyle = StyleSheet.create({
  wrap: { alignItems: "center", justifyContent: "center" },
  emoji: { fontSize: 18 },
});
