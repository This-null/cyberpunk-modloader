import { create } from "zustand";

const KEY = "cp2077-tour-done";

function seen(): boolean {
  try {
    return localStorage.getItem(KEY) === "1";
  } catch {
    return true;
  }
}

interface OnboardingState {
  open: boolean;
  step: number;
  go: (step: number) => void;
  show: () => void;
  finish: () => void;
}

export const useOnboarding = create<OnboardingState>((set) => ({
  open: !seen(),
  step: 0,
  go: (step) => set({ step }),
  show: () => set({ open: true, step: 0 }),
  finish: () => {
    try {
      localStorage.setItem(KEY, "1");
    } catch {
      // the tour then shows again next time
    }
    set({ open: false, step: 0 });
  },
}));

export const TOUR_STEPS = [
  { icon: "welcome", title: "onboard.title1", body: "onboard.body1" },
  { icon: "add", title: "onboard.title2", body: "onboard.body2" },
  { icon: "power", title: "onboard.title3", body: "onboard.body3" },
  { icon: "profiles", title: "onboard.title4", body: "onboard.body4" },
  { icon: "nexus", title: "onboard.title5", body: "onboard.body5" },
] as const;
