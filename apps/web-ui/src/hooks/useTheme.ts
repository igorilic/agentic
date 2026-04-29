import { useState, useEffect } from "react";

export type Theme = "light" | "dark";

export type UseThemeResult = {
  theme: Theme;
  setTheme: (t: Theme) => void;
  toggle: () => void;
};

export function useTheme(): UseThemeResult {
  const [theme, setThemeState] = useState<Theme>(() => {
    return localStorage.getItem("agentic.theme") === "dark" ? "dark" : "light";
  });

  useEffect(() => {
    if (theme === "dark") {
      document.documentElement.setAttribute("data-theme", "dark");
      localStorage.setItem("agentic.theme", "dark");
    } else {
      document.documentElement.removeAttribute("data-theme");
      localStorage.setItem("agentic.theme", "light");
    }
  }, [theme]);

  const setTheme = (t: Theme) => setThemeState(t);

  const toggle = () =>
    setThemeState((prev) => (prev === "light" ? "dark" : "light"));

  return { theme, setTheme, toggle };
}
