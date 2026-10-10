import { describe, expect, it } from "vitest";
import { extensionOf, iconKindForEntry } from "./language";

describe("extensionOf", () => {
  it("识别常见扩展名", () => {
    expect(extensionOf("App.vue")).toBe("vue");
    expect(extensionOf("C:\\项目\\主文件.TS")).toBe("ts");
    expect(extensionOf("noext")).toBe("");
    expect(extensionOf(".gitignore")).toBe("");
  });
});

describe("iconKindForEntry", () => {
  it("区分目录与文件", () => {
    expect(iconKindForEntry("src", true)).toBe("folder");
    expect(iconKindForEntry("main.rs", false)).toBe("rust");
    expect(iconKindForEntry("App.vue", false)).toBe("vue");
    expect(iconKindForEntry("index.ts", false)).toBe("code");
    expect(iconKindForEntry("style.css", false)).toBe("style");
    expect(iconKindForEntry("README.md", false)).toBe("markdown");
    expect(iconKindForEntry("Dockerfile", false)).toBe("config");
    expect(iconKindForEntry("logo.png", false)).toBe("image");
    expect(iconKindForEntry("unknown.xyzabc", false)).toBe("file");
  });
});
