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

let fomodHandler: (() => void) | null = null;

export function setFomodHandler(handler: (() => void) | null) {
  fomodHandler = handler;
}

export const useErrors = create<ErrorState>((set) => ({
  current: null,
  report: (e) => {
    const a = toAppError(e);
    if (a.code === "fomod" && fomodHandler) {
      fomodHandler();
      return;
    }
    set({ current: { code: a.code, technical: `${a.code}: ${a.message}` } });
  },
  dismiss: () => set({ current: null }),
}));
