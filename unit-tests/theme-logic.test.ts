import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  DEFAULT_THEME_ID,
  DRAGON_BALL_THEMES,
  THEME_LIST,
  THEME_STORAGE_KEY,
  getThemeCssVariables,
  getThemeDefinition,
  getThemeTerminalColors,
  isThemeId,
  resolveTheme,
  type ThemeId,
} from "../src/lib/theme-logic.ts";

describe("theme-logic", () => {
  describe("constants and defaults", () => {
    it("has trunks as the default theme id", () => {
      assert.equal(DEFAULT_THEME_ID, "trunks");
    });

    it("uses trunks.theme as the localStorage key", () => {
      assert.equal(THEME_STORAGE_KEY, "trunks.theme");
    });

    it("contains exactly 5 Dragon Ball themes in THEME_LIST in proper order", () => {
      assert.equal(THEME_LIST.length, 5);
      const ids = THEME_LIST.map((t) => t.id);
      assert.deepEqual(ids, ["trunks", "goku", "vegeta", "bulma", "piccolo"]);
    });
  });

  describe("resolveTheme and isThemeId", () => {
    it("recognizes valid theme ids", () => {
      assert.equal(isThemeId("trunks"), true);
      assert.equal(isThemeId("goku"), true);
      assert.equal(isThemeId("vegeta"), true);
      assert.equal(isThemeId("bulma"), true);
      assert.equal(isThemeId("piccolo"), true);
      assert.equal(isThemeId("frieza"), false);
      assert.equal(isThemeId(123), false);
      assert.equal(isThemeId(null), false);
      assert.equal(isThemeId(undefined), false);
    });

    it("resolves valid themes directly", () => {
      assert.equal(resolveTheme("trunks"), "trunks");
      assert.equal(resolveTheme("goku"), "goku");
      assert.equal(resolveTheme("vegeta"), "vegeta");
      assert.equal(resolveTheme("bulma"), "bulma");
      assert.equal(resolveTheme("piccolo"), "piccolo");
    });

    it("normalizes case and surrounding whitespace", () => {
      assert.equal(resolveTheme("  Goku  "), "goku");
      assert.equal(resolveTheme("VEGETA"), "vegeta");
      assert.equal(resolveTheme("Bulma"), "bulma");
      assert.equal(resolveTheme(" Piccolo "), "piccolo");
    });

    it("falls back safely to default theme on invalid or missing inputs", () => {
      assert.equal(resolveTheme(null), "trunks");
      assert.equal(resolveTheme(undefined), "trunks");
      assert.equal(resolveTheme(""), "trunks");
      assert.equal(resolveTheme("invalid-theme"), "trunks");
      assert.equal(resolveTheme(42), "trunks");
      assert.equal(resolveTheme({}), "trunks");
    });
  });

  describe("palette integrity and color specifications", () => {
    const expectedThemes: Record<
      ThemeId,
      { name: string; dotColor: string; accentPrimary: string }
    > = {
      trunks: { name: "Trunks", dotColor: "#9161D4", accentPrimary: "#9448f8" },
      goku: { name: "Goku", dotColor: "#E44E4B", accentPrimary: "#e44e4b" },
      vegeta: { name: "Vegeta", dotColor: "#3D59BE", accentPrimary: "#3d59be" },
      bulma: { name: "Bulma", dotColor: "#45B4D6", accentPrimary: "#45b4d6" },
      piccolo: { name: "Piccolo", dotColor: "#57A461", accentPrimary: "#57a461" },
    };

    for (const [id, expected] of Object.entries(expectedThemes) as [ThemeId, typeof expectedThemes[ThemeId]][]) {
      it(`theme ${id} satisfies design specifications`, () => {
        const theme = DRAGON_BALL_THEMES[id];
        assert.ok(theme, `Theme ${id} must exist`);
        assert.equal(theme.id, id);
        assert.equal(theme.name, expected.name);
        assert.equal(theme.dotColor.toUpperCase(), expected.dotColor.toUpperCase());
        assert.equal(theme.accentPrimary.toLowerCase(), expected.accentPrimary.toLowerCase());

        // Check required fields
        assert.ok(theme.accentHover, "accentHover must be defined");
        assert.ok(theme.accentMuted, "accentMuted must be defined");
        assert.ok(theme.accentBorder, "accentBorder must be defined");
        assert.ok(theme.accentSubtle, "accentSubtle must be defined");

        // Check terminal sub-object
        assert.ok(theme.terminal, "terminal must be defined");
        assert.ok(theme.terminal.cursor, "terminal.cursor must be defined");
        assert.ok(theme.terminal.selectionBackground, "terminal.selectionBackground must be defined");
        assert.ok(theme.terminal.magenta, "terminal.magenta must be defined");
        assert.ok(theme.terminal.brightMagenta, "terminal.brightMagenta must be defined");
      });
    }
  });

  describe("getThemeDefinition and helpers", () => {
    it("returns theme definition for valid and invalid inputs", () => {
      const goku = getThemeDefinition("goku");
      assert.equal(goku.id, "goku");
      assert.equal(goku.name, "Goku");

      const fallback = getThemeDefinition("unknown");
      assert.equal(fallback.id, "trunks");
    });

    it("generates correct CSS variables mapping", () => {
      const gokuVars = getThemeCssVariables("goku");
      assert.equal(gokuVars["--accent-primary"], "#e44e4b");
      assert.equal(gokuVars["--accent-hover"], "#ef6e6c");
      assert.equal(gokuVars["--accent-muted"], "rgba(228, 78, 75, 0.15)");
      assert.equal(gokuVars["--accent-border"], "rgba(228, 78, 75, 0.3)");
      assert.equal(gokuVars["--accent-subtle"], "rgba(228, 78, 75, 0.4)");
      assert.equal(gokuVars["--accent-dot"], "#E44E4B");
    });

    it("generates correct terminal colors mapping", () => {
      const vegetaTerminal = getThemeTerminalColors("vegeta");
      assert.equal(vegetaTerminal.cursor, "#3d59be");
      assert.equal(vegetaTerminal.selectionBackground, "#1a2652");
      assert.equal(vegetaTerminal.magenta, "#3d59be");
      assert.equal(vegetaTerminal.brightMagenta, "#5573d8");
    });
  });
});
