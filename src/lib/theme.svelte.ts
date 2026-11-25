import {
  DEFAULT_THEME_ID,
  THEME_STORAGE_KEY,
  getThemeCssVariables,
  getThemeDefinition,
  resolveTheme,
  type ThemeColors,
  type ThemeId,
} from "./theme-logic";

export function applyThemeToDocument(themeOrId: ThemeColors | ThemeId): void {
  if (typeof document === "undefined") return;
  const theme = typeof themeOrId === "string" ? getThemeDefinition(themeOrId) : themeOrId;
  const root = document.documentElement;
  root.setAttribute("data-theme", theme.id);
  const vars = getThemeCssVariables(theme);
  for (const [key, val] of Object.entries(vars)) {
    root.style.setProperty(key, val);
  }
}

export type ThemeChangeListener = (theme: ThemeColors) => void;

class ThemeStore {
  public currentThemeId = $state<ThemeId>(DEFAULT_THEME_ID);
  public currentTheme = $derived<ThemeColors>(getThemeDefinition(this.currentThemeId));

  private listeners = new Set<ThemeChangeListener>();
  private initialized = false;

  public init(): void {
    if (this.initialized) return;
    this.initialized = true;

    let savedTheme: string | null = null;
    if (typeof localStorage !== "undefined") {
      try {
        savedTheme = localStorage.getItem(THEME_STORAGE_KEY);
      } catch (err) {
        console.warn("Failed to read theme from localStorage:", err);
      }
    }

    const resolved = resolveTheme(savedTheme);
    this.currentThemeId = resolved;
    applyThemeToDocument(this.currentTheme);
  }

  public setTheme(id: unknown): void {
    const resolved = resolveTheme(id);
    this.currentThemeId = resolved;
    if (typeof localStorage !== "undefined") {
      try {
        localStorage.setItem(THEME_STORAGE_KEY, resolved);
      } catch (err) {
        console.warn("Failed to save theme to localStorage:", err);
      }
    }
    applyThemeToDocument(this.currentTheme);
    this.notifyListeners(this.currentTheme);
  }

  public subscribeThemeChanges(listener: ThemeChangeListener): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  private notifyListeners(theme: ThemeColors): void {
    for (const listener of this.listeners) {
      try {
        listener(theme);
      } catch (err) {
        console.error("Error in theme change listener:", err);
      }
    }
  }
}

export const themeStore = new ThemeStore();
