import { lazy, Suspense } from "react";
import type { StartRunFormProps } from "./StartRunFormInner";

// Vite static-folds `import.meta.env.DEV` at build time. The lazy()
// call is only emitted in dev builds, so prod bundles never reference
// StartRunFormInner and Rollup eliminates the file entirely.
//
// Tests that verify the DEV=false path must call vi.resetModules() and
// re-import this module after stubbing the env — see StartRunForm.test.tsx.
// Vitest's per-file module registry means the module-scope evaluation is
// isolated per test file, but within a file a vi.resetModules() + dynamic
// import is needed to re-evaluate this assignment with a different DEV value.
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
