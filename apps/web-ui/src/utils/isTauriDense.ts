export function isTauriDense(): boolean {
  if (
    typeof window !== "undefined" &&
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
  ) {
    return true;
  }
  if (import.meta.env.TAURI === "1") {
    return true;
  }
  return false;
}
