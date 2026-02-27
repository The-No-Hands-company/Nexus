/**
 * BadgesBar — renders a horizontal row of badge icons for a given user.
 * Used in UserProfileCard and member hover-cards.
 *
 * v0.15 Community Ecosystem
 */
import { useEffect } from "react";
import { useStore } from "../store";

interface Props {
  userId: string;
}

const BADGE_EMOJI: Record<string, string> = {
  early_adopter: "🌟",
  active_contributor: "⚡",
  verified_developer: "🔧",
  server_booster: "💎",
  partner: "🤝",
  hypesquad: "🏡",
  moderator: "🛡️",
  staff: "⚙️",
  custom: "🏅",
};

const BADGE_LABEL: Record<string, string> = {
  early_adopter: "Early Adopter",
  active_contributor: "Active Contributor",
  verified_developer: "Verified Developer",
  server_booster: "Server Booster",
  partner: "Partnered Server Member",
  hypesquad: "HypeSquad Member",
  moderator: "Certified Moderator",
  staff: "Nexus Staff",
  custom: "Custom Badge",
};

export default function BadgesBar({ userId }: Props) {
  const { userBadges, loadUserBadges } = useStore();
  const badges = userBadges[userId] ?? [];

  useEffect(() => {
    if (!userBadges[userId]) {
      loadUserBadges(userId);
    }
  }, [userId, userBadges, loadUserBadges]);

  if (badges.length === 0) return null;

  return (
    <div className="flex items-center gap-1 flex-wrap" role="list" aria-label="User badges">
      {badges.map((badge) => (
        <div
          key={badge.id}
          role="listitem"
          title={badge.label ?? BADGE_LABEL[badge.badgeType] ?? badge.badgeType}
          className="relative group"
        >
          {badge.iconUrl ? (
            <img
              src={badge.iconUrl}
              alt={badge.label ?? badge.badgeType}
              className="w-5 h-5 rounded-full object-cover ring-1 ring-bg-600/60"
            />
          ) : (
            <span className="text-base leading-none select-none" aria-hidden="true">
              {BADGE_EMOJI[badge.badgeType] ?? "🏅"}
            </span>
          )}
          {/* Tooltip */}
          <div
            className="pointer-events-none absolute bottom-full left-1/2 -translate-x-1/2 mb-1.5 bg-bg-900 text-fg text-[10px] rounded px-1.5 py-0.5 whitespace-nowrap shadow-lg ring-1 ring-bg-600/40 opacity-0 group-hover:opacity-100 transition-opacity z-50"
            role="tooltip"
          >
            {badge.label ?? BADGE_LABEL[badge.badgeType] ?? badge.badgeType}
          </div>
        </div>
      ))}
    </div>
  );
}
