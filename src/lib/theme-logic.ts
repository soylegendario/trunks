export type ThemeId = "trunks" | "goku" | "vegeta" | "bulma" | "piccolo";

export interface ThemeColors {
  id: ThemeId;
  name: string;
  dotColor: string;
  accentPrimary: string;
  accentHover: string;
  accentMuted: string;
  accentBorder: string;
  accentSubtle: string;
  terminal: {
    cursor: string;
    selectionBackground: string;
    magenta: string;
    brightMagenta: string;
  };
}

export const THEME_STORAGE_KEY = "trunks.theme";
export const DEFAULT_THEME_ID: ThemeId = "trunks";

export const DRAGON_BALL_THEMES: Record<ThemeId, ThemeColors> = {
  trunks: {
    id: "trunks",
    name: "Trunks",
    dotColor: "#9161D4",
    accentPrimary: "#9448f8",
    accentHover: "#aa6dfb",
    accentMuted: "rgba(148, 72, 248, 0.15)",
    accentBorder: "rgba(148, 72, 248, 0.3)",
    accentSubtle: "rgba(148, 72, 248, 0.4)",
    terminal: {
      cursor: "#9448f8",
      selectionBackground: "#3a1d5e",
      magenta: "#9448f8",
      brightMagenta: "#aa6dfb",
    },
  },
  goku: {
    id: "goku",
    name: "Goku",
    dotColor: "#E44E4B",
    accentPrimary: "#e44e4b",
    accentHover: "#ef6e6c",
    accentMuted: "rgba(228, 78, 75, 0.15)",
    accentBorder: "rgba(228, 78, 75, 0.3)",
    accentSubtle: "rgba(228, 78, 75, 0.4)",
    terminal: {
      cursor: "#e44e4b",
      selectionBackground: "#5a1e1d",
      magenta: "#e44e4b",
      brightMagenta: "#ef6e6c",
    },
  },
  vegeta: {
    id: "vegeta",
    name: "Vegeta",
    dotColor: "#3D59BE",
    accentPrimary: "#3d59be",
    accentHover: "#5573d8",
    accentMuted: "rgba(61, 89, 190, 0.15)",
    accentBorder: "rgba(61, 89, 190, 0.3)",
    accentSubtle: "rgba(61, 89, 190, 0.4)",
    terminal: {
      cursor: "#3d59be",
      selectionBackground: "#1a2652",
      magenta: "#3d59be",
      brightMagenta: "#5573d8",
    },
  },
  bulma: {
    id: "bulma",
    name: "Bulma",
    dotColor: "#45B4D6",
    accentPrimary: "#45b4d6",
    accentHover: "#66c8e5",
    accentMuted: "rgba(69, 180, 214, 0.15)",
    accentBorder: "rgba(69, 180, 214, 0.3)",
    accentSubtle: "rgba(69, 180, 214, 0.4)",
    terminal: {
      cursor: "#45b4d6",
      selectionBackground: "#1d4c5b",
      magenta: "#45b4d6",
      brightMagenta: "#66c8e5",
    },
  },
  piccolo: {
    id: "piccolo",
    name: "Piccolo",
    dotColor: "#57A461",
    accentPrimary: "#57a461",
    accentHover: "#72ba7b",
    accentMuted: "rgba(87, 164, 97, 0.15)",
    accentBorder: "rgba(87, 164, 97, 0.3)",
    accentSubtle: "rgba(87, 164, 97, 0.4)",
    terminal: {
      cursor: "#57a461",
      selectionBackground: "#224426",
      magenta: "#57a461",
      brightMagenta: "#72ba7b",
    },
  },
};

export const THEME_LIST: readonly ThemeColors[] = [
  DRAGON_BALL_THEMES.trunks,
  DRAGON_BALL_THEMES.goku,
  DRAGON_BALL_THEMES.vegeta,
  DRAGON_BALL_THEMES.bulma,
  DRAGON_BALL_THEMES.piccolo,
];

export function isThemeId(value: unknown): value is ThemeId {
  return typeof value === "string" && Object.prototype.hasOwnProperty.call(DRAGON_BALL_THEMES, value);
}

export function resolveTheme(raw: unknown): ThemeId {
  if (typeof raw !== "string") {
    return DEFAULT_THEME_ID;
  }
  const normalized = raw.trim().toLowerCase();
  return isThemeId(normalized) ? normalized : DEFAULT_THEME_ID;
}

export function getThemeDefinition(idOrRaw: unknown): ThemeColors {
  const themeId = resolveTheme(idOrRaw);
  return DRAGON_BALL_THEMES[themeId];
}

export function getThemeCssVariables(themeOrId: ThemeColors | ThemeId | string): Record<string, string> {
  const theme =
    typeof themeOrId === "object" && themeOrId !== null && "accentPrimary" in themeOrId
      ? (themeOrId as ThemeColors)
      : getThemeDefinition(themeOrId);
  return {
    "--accent-primary": theme.accentPrimary,
    "--accent-hover": theme.accentHover,
    "--accent-muted": theme.accentMuted,
    "--accent-border": theme.accentBorder,
    "--accent-subtle": theme.accentSubtle,
    "--accent-dot": theme.dotColor,
  };
}

export function getThemeTerminalColors(themeOrId: ThemeColors | ThemeId | string) {
  const theme =
    typeof themeOrId === "object" && themeOrId !== null && "terminal" in themeOrId
      ? (themeOrId as ThemeColors)
      : getThemeDefinition(themeOrId);
  return {
    cursor: theme.terminal.cursor,
    selectionBackground: theme.terminal.selectionBackground,
    magenta: theme.terminal.magenta,
    brightMagenta: theme.terminal.brightMagenta,
  };
}
