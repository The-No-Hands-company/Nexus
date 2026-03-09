import { useEffect, useState } from "react";
import { useStore } from "../store";
import { invoke } from "../invoke";
import type { OnboardingProgress } from "../store";

const ONBOARDING_STEPS = [
  { key: "create_server", label: "Create your first server", description: "Set up a community space for your team or friends." },
  { key: "invite_members", label: "Invite members", description: "Share an invite link or send email invitations." },
  { key: "customize_profile", label: "Customize your profile", description: "Add an avatar and display name." },
  { key: "pick_theme", label: "Pick a theme", description: "Choose a color theme that suits you." },
  { key: "send_message", label: "Send your first message", description: "Say hi in any channel!" },
];

/**
 * OnboardingWizard — First-run guided setup for new users.
 * Shows a step-by-step checklist that persists progress.
 */
export default function OnboardingWizard() {
  const { onboardingProgress, loadOnboardingProgress, dismissOnboarding } = useStore();
  const [completedSteps, setCompletedSteps] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    loadOnboardingProgress();
  }, [loadOnboardingProgress]);

  useEffect(() => {
    if (onboardingProgress) {
      const steps = Array.isArray(onboardingProgress.completedSteps)
        ? (onboardingProgress.completedSteps as string[])
        : [];
      setCompletedSteps(steps);
    }
  }, [onboardingProgress]);

  // Dismissed — don't render
  if (onboardingProgress?.dismissed) return null;

  // All steps done — auto-dismiss
  const allDone = ONBOARDING_STEPS.every((s) => completedSteps.includes(s.key));

  const handleToggleStep = async (key: string) => {
    setSaving(true);
    try {
      const updated = completedSteps.includes(key)
        ? completedSteps.filter((s) => s !== key)
        : [...completedSteps, key];
      const result = await invoke<OnboardingProgress>("update_onboarding_progress", {
        completedSteps: updated,
      });
      setCompletedSteps(
        Array.isArray(result.completedSteps)
          ? (result.completedSteps as string[])
          : [],
      );
    } catch (e) {
      console.error("toggle step error", e);
    } finally {
      setSaving(false);
    }
  };

  const progress = ONBOARDING_STEPS.length > 0
    ? Math.round((completedSteps.length / ONBOARDING_STEPS.length) * 100)
    : 0;

  return (
    <div className="bg-gray-800/80 border border-gray-700 rounded-lg p-5 max-w-md mx-auto">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-bold text-white">Welcome to Nexus!</h2>
        <button
          onClick={dismissOnboarding}
          className="text-xs text-gray-500 hover:text-gray-300"
          title="Dismiss"
        >
          ✕
        </button>
      </div>

      {/* Progress bar */}
      <div className="w-full bg-gray-700 h-2 rounded-full mb-4 overflow-hidden">
        <div
          className="h-full bg-indigo-500 rounded-full transition-all"
          style={{ width: `${progress}%` }}
        />
      </div>
      <p className="text-xs text-gray-400 mb-4">{progress}% complete</p>

      {allDone ? (
        <div className="text-center">
          <p className="text-green-400 font-semibold mb-2">All set! 🎉</p>
          <p className="text-sm text-gray-400 mb-3">You&apos;re ready to go.</p>
          <button
            onClick={dismissOnboarding}
            className="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded text-sm"
          >
            Get Started
          </button>
        </div>
      ) : (
        <ul className="space-y-3">
          {ONBOARDING_STEPS.map((step) => {
            const done = completedSteps.includes(step.key);
            return (
              <li key={step.key} className="flex items-start gap-3">
                <button
                  onClick={() => handleToggleStep(step.key)}
                  disabled={saving}
                  className={`mt-0.5 w-5 h-5 rounded flex items-center justify-center flex-shrink-0 border transition-colors ${
                    done
                      ? "bg-indigo-600 border-indigo-500 text-white"
                      : "bg-gray-800 border-gray-600 hover:border-gray-500"
                  }`}
                >
                  {done && (
                    <svg className="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M5 13l4 4L19 7" />
                    </svg>
                  )}
                </button>
                <div>
                  <p className={`text-sm font-medium ${done ? "text-gray-500 line-through" : "text-white"}`}>
                    {step.label}
                  </p>
                  <p className="text-xs text-gray-500">{step.description}</p>
                </div>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
