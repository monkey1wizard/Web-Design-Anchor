type Locale = "zh-TW" | "en-US";
type Severity = "error" | "advisory";

type Finding = {
  filePath: string;
  line: number;
  column: number;
  rule: string;
  severity: Severity;
  message: string;
};

type PathRule = {
  locale: Locale;
  matches: (relativePath: string) => boolean;
};

const scriptDirectory = import.meta.dirname ?? Deno.cwd();
const repositoryRoot = Deno.realPathSync(`${scriptDirectory}/../..`);
const installCommand = "deno install --config tools/writing/deno.json";

const pathRules: PathRule[] = [
  { locale: "zh-TW", matches: (p) => /^docs\/[^/]+\.md$/.test(p) },
  { locale: "zh-TW", matches: (p) => /^docs\/adr\/[^/]+\.md$/.test(p) },
  { locale: "zh-TW", matches: (p) => /^docs\/design-systems\/[^/]+\.md$/.test(p) },
  { locale: "zh-TW", matches: (p) => p === "skills/wda/SKILL.md" },
  { locale: "zh-TW", matches: (p) => /^skills\/wda\/references\/[^/]+\.md$/.test(p) },
  { locale: "zh-TW", matches: (p) => ["README.md", "SECURITY.md", "AGENTS.md", ".dev/project.md"].includes(p) },
  { locale: "en-US", matches: (p) => /^docs\/templates\/generated-project\/[^/]+\.md$/.test(p) },
  { locale: "en-US", matches: (p) => p === "CHANGELOG.md" },
  { locale: "en-US", matches: (p) => p === "src/builtins/wda_minimal/docs/design.md" },
];

function relativePath(filePath: string): string {
  const absolute = Deno.realPathSync(filePath).replaceAll("\\", "/");
  const root = repositoryRoot.replaceAll("\\", "/").replace(/\/+$/, "");
  return absolute.startsWith(`${root}/`) ? absolute.slice(root.length + 1) : absolute;
}

function absolutePath(filePath: string): string {
  return /^[A-Za-z]:[\\/]/.test(filePath) || filePath.startsWith("/")
    ? filePath
    : `${repositoryRoot}/${filePath}`;
}

async function markdownFiles(directory: string): Promise<string[]> {
  const files: string[] = [];
  for await (const entry of Deno.readDir(directory)) {
    const entryPath = `${directory}/${entry.name}`;
    if (entry.isDirectory) {
      files.push(...await markdownFiles(entryPath));
    } else if (entry.isFile && entry.name.endsWith(".md")) {
      files.push(entryPath);
    }
  }
  return files.sort((left, right) => left.localeCompare(right));
}

function configuredLocales(filePath: string): Locale[] {
  const path = relativePath(filePath);
  return pathRules.filter((rule) => rule.matches(path)).map((rule) => rule.locale);
}

function configPath(locale: Locale): string {
  return `${scriptDirectory}/${locale}.textlintrc.json`;
}

function ruleSeverity(rule: string): Severity {
  const normalized = rule.toLowerCase();
  return normalized === "prh" || normalized.includes("no-unmatched-pair") ? "error" : "advisory";
}

function isFrozenHistory(filePath: string, line: number, content: string): boolean {
  if (!relativePath(filePath).startsWith("docs/adr/")) return false;
  const headingLine = content.split(/\r?\n/).findIndex((value) => value.trim() === "## 狀態沿革");
  return headingLine >= 0 && line > headingLine + 1;
}

function printFinding(finding: Finding): void {
  console.log(`${finding.filePath}:${finding.line}:${finding.column} ${finding.rule} ${finding.severity}: ${finding.message}`);
}

function toolFailure(message: string): number {
  console.error(`Tool failure: ${message}`);
  return 2;
}

async function verifyInputs(locales: Locale[]): Promise<void> {
  for (const locale of locales) {
    const config = configPath(locale);
    const dictionary = `${scriptDirectory}/prh.yml`;
    try {
      await Deno.stat(config);
      await Deno.stat(dictionary);
      await Deno.stat(`${scriptDirectory}/node_modules`);
    } catch {
      throw new Error(`${locale} configuration, prh dictionary, or cached dependencies are missing. Run ${installCommand} once.`);
    }
  }
}

async function loadLinter(locale: Locale) {
  try {
    const textlint = await import("textlint");
    const descriptor = await textlint.loadTextlintrc({
      configFilePath: configPath(locale),
      node_modulesDir: `${scriptDirectory}/node_modules`,
    });
    const loadedDescriptor = descriptor as unknown as { args?: { rules?: Array<{ ruleId?: string }> } };
    const ruleIds = new Set((loadedDescriptor.args?.rules ?? []).map((rule) => rule.ruleId));
    const expectedRules = locale === "zh-TW"
      ? ["prh", "@textlint-rule/no-unmatched-pair", "sentence-length"]
      : ["prh", "@textlint-rule/no-unmatched-pair", "write-good"];
    if (expectedRules.some((ruleId) => !ruleIds.has(ruleId))) {
      throw new Error(`expected rules were not loaded: ${expectedRules.filter((ruleId) => !ruleIds.has(ruleId)).join(", ")}`);
    }
    return textlint.createLinter({ descriptor, cwd: repositoryRoot });
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    throw new Error(`textlint could not load its ${locale} configuration (${detail}). Run ${installCommand} once.`);
  }
}

async function lintFile(filePath: string, locale: Locale, linter: { lintText: (text: string, filePath: string) => Promise<{ messages: Array<{ ruleId?: string; message: string; line?: number; column?: number }> }> }): Promise<Finding[]> {
  const content = await Deno.readTextFile(filePath);
  const result = await linter.lintText(content, filePath);
  return result.messages.map((message) => {
    const rule = message.ruleId ?? "unknown";
    const frozen = isFrozenHistory(filePath, message.line ?? 1, content);
    return {
      filePath: relativePath(filePath),
      line: message.line ?? 1,
      column: message.column ?? (message as { loc?: { start?: { column?: number } } }).loc?.start?.column ?? 1,
      rule,
      severity: frozen ? "advisory" : ruleSeverity(rule),
      message: frozen ? `${message.message} (frozen history)` : message.message,
    };
  });
}

function parseArguments(args: string[]): { required: boolean; locale?: Locale; files: string[] } {
  if (args.length === 1 && args[0] === "--required") return { required: true, files: [] };
  if (args[0] !== "--locale" || !["zh-TW", "en-US"].includes(args[1] ?? "") || args[2] !== "--files" || args.length < 4) {
    throw new Error("Usage: --required or --locale <zh-TW|en-US> --files <paths...>");
  }
  return { required: false, locale: args[1] as Locale, files: args.slice(3) };
}

async function run(args: string[]): Promise<number> {
  let options: ReturnType<typeof parseArguments>;
  try {
    options = parseArguments(args);
  } catch (error) {
    return toolFailure(error instanceof Error ? error.message : String(error));
  }

  let files: Array<{ path: string; locale: Locale }>;
  try {
    if (options.required) {
      const candidates = [
        ...await markdownFiles(`${repositoryRoot}/docs`),
        ...await markdownFiles(`${repositoryRoot}/skills/wda`),
      ];
      const unmapped = candidates.filter((filePath) => configuredLocales(filePath).length !== 1);
      if (unmapped.length > 0) {
        throw new Error(`required Markdown file must match exactly one locale pattern: ${unmapped.map(relativePath).join(", ")}`);
      }
      files = candidates.map((path) => ({ path, locale: configuredLocales(path)[0] }));
      for (const rootFile of ["README.md", "SECURITY.md", "AGENTS.md", ".dev/project.md", "CHANGELOG.md", "src/builtins/wda_minimal/docs/design.md"]) {
        const path = absolutePath(rootFile);
        await Deno.stat(path);
        const locales = configuredLocales(path);
        if (locales.length !== 1) throw new Error(`required file must match exactly one locale pattern: ${rootFile}`);
        if (!files.some((file) => file.path === path)) files.push({ path, locale: locales[0] });
      }
    } else {
      files = options.files.map((file) => ({ path: absolutePath(file), locale: options.locale! }));
    }
    await verifyInputs([...new Set(files.map((file) => file.locale))]);
  } catch (error) {
    return toolFailure(error instanceof Error ? error.message : String(error));
  }

  const findings: Finding[] = [];
  try {
    const linters = new Map<Locale, Awaited<ReturnType<typeof loadLinter>>>();
    for (const file of files) {
      if (!linters.has(file.locale)) linters.set(file.locale, await loadLinter(file.locale));
      findings.push(...await lintFile(file.path, file.locale, linters.get(file.locale)!));
    }
  } catch (error) {
    return toolFailure(error instanceof Error ? error.message : String(error));
  }

  findings.sort((a, b) => a.filePath.localeCompare(b.filePath) || a.line - b.line || a.column - b.column || a.rule.localeCompare(b.rule) || a.message.localeCompare(b.message));
  findings.forEach(printFinding);
  return findings.some((finding) => finding.severity === "error") ? 1 : 0;
}

if (import.meta.main) {
  Deno.exit(await run(Deno.args));
}

export { run };
