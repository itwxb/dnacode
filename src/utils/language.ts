/** 按扩展名按需加载 CodeMirror 语言支持（动态 import，避免加载不需要的语言包）。 */

import type { Extension } from "@codemirror/state";

const cache = new Map<string, Extension | null>();

export function extensionOf(fileName: string): string {
  const base = fileName.split(/[\\/]/).pop() ?? fileName;
  const idx = base.lastIndexOf(".");
  if (idx <= 0) return "";
  return base.slice(idx + 1).toLowerCase();
}

async function load(ext: string): Promise<Extension | null> {
  switch (ext) {
    case "js":
    case "mjs":
    case "cjs": {
      const { javascript } = await import("@codemirror/lang-javascript");
      return javascript();
    }
    case "jsx": {
      const { javascript } = await import("@codemirror/lang-javascript");
      return javascript({ jsx: true });
    }
    case "ts":
    case "mts":
    case "cts": {
      const { javascript } = await import("@codemirror/lang-javascript");
      return javascript({ typescript: true });
    }
    case "tsx": {
      const { javascript } = await import("@codemirror/lang-javascript");
      return javascript({ typescript: true, jsx: true });
    }
    case "vue": {
      const { vue } = await import("@codemirror/lang-vue");
      return vue();
    }
    case "html":
    case "htm": {
      const { html } = await import("@codemirror/lang-html");
      return html();
    }
    case "css":
    case "scss":
    case "less": {
      const { css } = await import("@codemirror/lang-css");
      return css();
    }
    case "json":
    case "jsonc": {
      const { json } = await import("@codemirror/lang-json");
      return json();
    }
    case "md":
    case "markdown": {
      const { markdown } = await import("@codemirror/lang-markdown");
      return markdown();
    }
    case "py": {
      const { python } = await import("@codemirror/lang-python");
      return python();
    }
    case "rs": {
      const { rust } = await import("@codemirror/lang-rust");
      return rust();
    }
    case "java": {
      const { java } = await import("@codemirror/lang-java");
      return java();
    }
    case "php": {
      const { php } = await import("@codemirror/lang-php");
      return php();
    }
    case "yml":
    case "yaml": {
      const { yaml } = await import("@codemirror/lang-yaml");
      return yaml();
    }
    default:
      return null;
  }
}

/** 根据文件名加载语言扩展；未知类型返回 null（普通文本模式）。 */
export async function loadLanguageForFile(fileName: string): Promise<Extension | null> {
  const ext = extensionOf(fileName);
  if (cache.has(ext)) {
    return cache.get(ext) ?? null;
  }
  const lang = await load(ext);
  cache.set(ext, lang);
  return lang;
}

export type FileIconKind =
  | "folder"
  | "code"
  | "vue"
  | "web"
  | "style"
  | "data"
  | "config"
  | "markdown"
  | "python"
  | "rust"
  | "java"
  | "php"
  | "image"
  | "archive"
  | "lock"
  | "text"
  | "file";

/** 文件树图标归类（配合少量内置 SVG 使用）。 */
export function iconKindForEntry(name: string, isDir: boolean): FileIconKind {
  if (isDir) return "folder";
  const lower = name.toLowerCase();
  if (lower === "dockerfile" || lower === "makefile" || lower.startsWith(".env")) return "config";
  if (lower === ".gitignore" || lower === ".gitattributes") return "config";
  const ext = extensionOf(name);
  switch (ext) {
    case "js":
    case "mjs":
    case "cjs":
    case "jsx":
    case "ts":
    case "tsx":
    case "mts":
    case "cts":
      return "code";
    case "vue":
      return "vue";
    case "html":
    case "htm":
      return "web";
    case "css":
    case "scss":
    case "less":
      return "style";
    case "json":
    case "jsonc":
    case "toml":
    case "csv":
      return "data";
    case "yml":
    case "yaml":
    case "ini":
    case "conf":
    case "config":
      return "config";
    case "md":
    case "markdown":
      return "markdown";
    case "py":
      return "python";
    case "rs":
      return "rust";
    case "java":
      return "java";
    case "php":
      return "php";
    case "png":
    case "jpg":
    case "jpeg":
    case "gif":
    case "webp":
    case "ico":
    case "bmp":
    case "svg":
      return "image";
    case "zip":
    case "tar":
    case "gz":
    case "7z":
    case "rar":
      return "archive";
    case "lock":
      return "lock";
    case "txt":
    case "log":
      return "text";
    default:
      return "file";
  }
}
