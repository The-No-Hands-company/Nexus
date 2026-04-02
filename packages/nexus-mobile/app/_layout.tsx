import { Stack } from "expo-router";

export default function RootLayout() {
  return (
    <Stack
      screenOptions={{
        headerStyle: { backgroundColor: "#071327" },
        headerTintColor: "#d6f4ff",
        contentStyle: { backgroundColor: "#030a14" },
      }}
    />
  );
}
