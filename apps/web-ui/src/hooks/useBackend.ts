import { useState, useEffect } from "react";
import { type BackendKind, ALLOWED_BACKENDS } from "../slash/types";

export type { BackendKind };

const STORAGE_KEY = "agentic.backend";
const DEFAULT_BACKEND: BackendKind = "claude-code";

function readStoredBackend(): BackendKind {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored && (ALLOWED_BACKENDS as readonly string[]).includes(stored)) {
    return stored as BackendKind;
  }
  return DEFAULT_BACKEND;
}

export function useBackend(): {
  backend: BackendKind;
  setBackend: (b: BackendKind) => void;
} {
  const [backend, setBackendState] = useState<BackendKind>(readStoredBackend);

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, backend);
  }, [backend]);

  const setBackend = (b: BackendKind) => setBackendState(b);

  return { backend, setBackend };
}
