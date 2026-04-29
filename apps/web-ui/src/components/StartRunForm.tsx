import { lazy, Suspense } from "react";
import type { StartRunFormProps } from "./StartRunFormInner";

// Vite static-folds `import.meta.env.DEV` at build time. The lazy()
// call is only emitted in dev builds, so prod bundles never reference
// StartRunFormInner and Rollup eliminates the file entirely.
const StartRunFormInner = import.meta.env.DEV
  ? lazy(() => import("./StartRunFormInner"))
  : null;

export default function StartRunForm(props: StartRunFormProps) {
  if (!StartRunFormInner) return null;
  return (
    <Suspense fallback={null}>
      <StartRunFormInner {...props} />
    </Suspense>
  );
}
