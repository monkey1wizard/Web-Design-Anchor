type Locale = "zh-TW" | "en-US" | "ja";
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
const translationLocales = ["zh-TW", "ja"] as const;

const pathRules: PathRule[] = [
  { locale: "en-US", matches: (p) => p === "docs/architecture.md" },
  { locale: "en-US", matches: (p) => p === "docs/commands.md" },
  { locale: "en-US", matches: (p) => p === "docs/design-system.md" },
  { locale: "en-US", matches: (p) => p === "docs/generated-project.md" },
  { locale: "en-US", matches: (p) => p === "docs/glossary.md" },
  { locale: "en-US", matches: (p) => p === "docs/skill-spec.md" },
  { locale: "en-US", matches: (p) => /^docs\/adr\/[^/]+\.md$/.test(p) },
  { locale: "en-US", matches: (p) => /^docs\/design-systems\/[^/]+\.md$/.test(p) },
  ...translationLocales.map((locale) => ({ locale, matches: (p: string) => new RegExp(`^docs/i18n/${locale}/[^/]+\\.${locale}\\.md$`).test(p) })),
  { locale: "en-US", matches: (p) => p === "skills/wda/SKILL.md" },
  { locale: "en-US", matches: (p) => /^skills\/wda\/references\/[^/]+\.md$/.test(p) },
  { locale: "en-US", matches: (p) => ["README.md", "SECURITY.md"].includes(p) },
  { locale: "zh-TW", matches: (p) => ["AGENTS.md", ".dev/project.md"].includes(p) },
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
  const headingLine = content.split(/\r?\n/).findIndex((value) => value.trim() === "## Status History");
  return headingLine >= 0 && line > headingLine + 1;
}

function translationSourcePath(filePath: string): { expectedPath?: string; error?: string } {
  const name = filePath.split("/").at(-1)!;
  const locale = translationLocales.find((candidate) => name.endsWith(`.${candidate}.md`));
  if (!locale) return { error: `translation ${filePath} has an unsupported locale suffix` };
  const stem = name.slice(0, -`.${locale}.md`.length);
  if (stem === "README" || stem === "SECURITY") return { expectedPath: `${stem}.md` };
  const candidates = [`docs/${stem}.md`, `docs/design-systems/${stem}.md`];
  const existing = candidates.filter((candidate) => {
    try {
      Deno.statSync(absolutePath(candidate));
      return true;
    } catch (error) {
      if (error instanceof Deno.errors.NotFound) return false;
      throw error;
    }
  });
  if (existing.length !== 1) {
    return { error: `translation ${filePath} must have exactly one canonical source candidate, found ${existing.length}` };
  }
  return { expectedPath: existing[0] };
}

async function translationFreshnessFinding(filePath: string): Promise<Finding | undefined> {
  const path = relativePath(filePath);
  const content = await Deno.readTextFile(filePath);
  const lines = content.split(/\r?\n/);
  const frontMatterLines: string[] = [];
  if (lines[0]?.trim() === "---") {
    for (const line of lines.slice(1)) {
      if (line.trim() === "---") break;
      frontMatterLines.push(line);
    }
  }
  const sourceLine = frontMatterLines.find((line) => /^source\s*:/.test(line));
  const hashLine = frontMatterLines.find((line) => /^source_sha256\s*:/.test(line));
  const source = sourceLine?.match(/^source\s*:\s*(.*?)\s*$/)?.[1];
  const hash = hashLine?.match(/^source_sha256\s*:\s*([0-9a-fA-F]{64})\s*$/)?.[1];
  const finding = (message: string): Finding => ({ filePath: path, line: 1, column: 1, rule: "translation-freshness", severity: "error", message });
  if (!sourceLine || !source || !hashLine || !hash) return finding(`translation ${path} is missing a valid source or source_sha256 front matter key`);

  const derived = translationSourcePath(path);
  if (derived.error) return finding(derived.error);
  if (source !== derived.expectedPath) return finding(`translation ${path} source must be ${derived.expectedPath}, got ${source}`);
  const sourceFile = absolutePath(derived.expectedPath!);
  try {
    await Deno.stat(sourceFile);
  } catch (error) {
    if (error instanceof Deno.errors.NotFound) return finding(`translation ${path} source file is missing: ${derived.expectedPath}`);
    throw error;
  }
  const sourceContent = (await Deno.readTextFile(sourceFile)).replaceAll("\r\n", "\n");
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(sourceContent));
  const computedHash = Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
  if (hash.toLowerCase() !== computedHash) return finding(`translation ${path} is stale; computed source_sha256 ${computedHash}`);
  return undefined;
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
    try {
      const configText = await Deno.readTextFile(config);
      const parsed = JSON.parse(configText) as { rules?: { prh?: { rulePaths?: string[] } } };
      const dictionaryName = parsed.rules?.prh?.rulePaths?.[0];
      if (!dictionaryName) throw new Error(`${locale} configuration does not name a prh dictionary`);
      const dictionary = `${scriptDirectory}/${dictionaryName.replace(/^\.\//, "")}`;
      try {
        await Deno.stat(dictionary);
      } catch (error) {
        if (error instanceof Deno.errors.NotFound) throw new Error(`${locale} dictionary is missing: ${dictionaryName.replace(/^\.\//, "")}. Run ${installCommand} once.`);
        throw error;
      }
      await Deno.stat(`${scriptDirectory}/node_modules`);
    } catch (error) {
      if (error instanceof Error && error.message.includes("dictionary is missing:")) throw error;
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
    const expectedRules = locale === "en-US"
      ? ["prh", "@textlint-rule/no-unmatched-pair", "write-good"]
      : ["prh", "@textlint-rule/no-unmatched-pair", "sentence-length"];
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
  if (args[0] !== "--locale" || !["zh-TW", "en-US", "ja"].includes(args[1] ?? "") || args[2] !== "--files" || args.length < 4) {
    throw new Error("Usage: --required or --locale <zh-TW|en-US|ja> --files <paths...>");
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
  const findings: Finding[] = [];
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
      for (const file of files.filter((candidate) => translationLocales.some((locale) => new RegExp(`^docs/i18n/${locale}/[^/]+\\.${locale}\\.md$`).test(relativePath(candidate.path))))) {
        const freshnessFinding = await translationFreshnessFinding(file.path);
        if (freshnessFinding) findings.push(freshnessFinding);
      }
    } else {
      files = options.files.map((file) => ({ path: absolutePath(file), locale: options.locale! }));
    }
    await verifyInputs([...new Set(files.map((file) => file.locale))]);
  } catch (error) {
    return toolFailure(error instanceof Error ? error.message : String(error));
  }

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
