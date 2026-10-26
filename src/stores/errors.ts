import { create } from "zustand";
import { en, type MessageKey } from "../i18n/en";
import { toAppError } from "../lib/ipc";

export interface ShownError {
  code: string;
  technical: string;
}

interface ErrorState {
  current: ShownError | null;
  report: (e: unknown) => void;
  dismiss: () => void;
}

export function errorMessageKey(code: string): MessageKey | null {
  const key = `err.${code}`;
  return key in en ? (key as MessageKey) : null;
}

export const useErrors = create<ErrorState>((set) => ({
  current: null,
  report: (e) => {
    const a = toAppError(e);
    set({ current: { code: a.code, technical: `${a.code}: ${a.message}` } });
  },
  dismiss: () => set({ current: null }),
}));
