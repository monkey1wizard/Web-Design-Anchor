const deno = Deno.execPath();
const scriptDir = (import.meta.dirname ?? Deno.cwd()).replaceAll("\\", "/");
const repoRoot = Deno.realPathSync(`${scriptDir}/../..`).replaceAll("\\", "/");
const checkScript = `${scriptDir}/check.ts`;
const denoConfig = `${scriptDir}/deno.json`;

function assertEquals<T>(actual: T, expected: T, msg?: string): void {
  if (actual !== expected) {
    throw new Error(msg ?? `Expected ${String(expected)}, got ${String(actual)}`);
  }
}

function assertStringIncludes(actual: string, expected: string): void {
  if (!actual.includes(expected)) {
    throw new Error(`Expected output to include "${expected}", but got:\n${actual}`);
  }
}

function assertNotIncludes(actual: string, unexpected: string): void {
  if (actual.includes(unexpected)) {
    throw new Error(`Expected output NOT to include "${unexpected}", but got:\n${actual}`);
  }
}

async function execute(
  args: string[],
  options?: {
    cwd?: string;
    script?: string;
    env?: Record<string, string>;
    permissions?: string[];
  },
): Promise<{ code: number; stdout: string; stderr: string; output: string }> {
  const perms = options?.permissions ?? [
    "--allow-read",
    "--allow-env",
    "--allow-sys",
  ];
  const targetScript = options?.script ?? checkScript;
  const command = new Deno.Command(deno, {
    args: [
      "run",
      "--cached-only",
      ...perms,
      "--config",
      denoConfig,
      targetScript,
      ...args,
    ],
    cwd: options?.cwd ?? repoRoot,
    env: options?.env,
    stdout: "piped",
    stderr: "piped",
  });
  const result = await command.output();
  const stdout = new TextDecoder().decode(result.stdout);
  const stderr = new TextDecoder().decode(result.stderr);
  return {
    code: result.code,
    stdout,
    stderr,
    output: `${stdout}${stderr}`,
  };
}

Deno.test("C-01: forbidden term in prose is reported, but ignored in code span and code block (exit 1)", async () => {
  const tempFile = await Deno.makeTempFile({ suffix: ".md" });
  const backtick = "`";
  const content = [
    "這個倉庫是測試用的標題。",
    "",
    `這是一段行內程式碼 ${backtick}倉庫${backtick} 不應該被當作錯誤。`,
    "",
    `${backtick}${backtick}${backtick}`,
    "倉庫",
    `${backtick}${backtick}${backtick}`,
    "",
  ].join("\n");
  await Deno.writeTextFile(tempFile, content);

  try {
    const result = await execute(["--locale", "zh-TW", "--files", tempFile]);
    assertEquals(result.code, 1, "Expected exit code 1 for hard error in prose");
    assertStringIncludes(result.output, "prh error: 倉庫 => 儲存庫");
    assertStringIncludes(result.output, ":1:");
    assertNotIncludes(result.output, ":3:");
    assertNotIncludes(result.output, ":6:");
  } finally {
    await Deno.remove(tempFile);
  }
});

Deno.test("C-03: required-set run maps files to correct locale configuration", async () => {
  const zhProbe = `${repoRoot}/docs/writing-locale-probe.md`;
  const enProbe = `${repoRoot}/docs/templates/generated-project/writing-locale-probe.md`;
  await Deno.writeTextFile(zhProbe, "這個設計系統是測試用的。\n");
  await Deno.writeTextFile(enProbe, "This document was utilized by the system.\n");

  try {
    const enResult = await execute(["--locale", "en-US", "--files", "docs/templates/generated-project/writing-locale-probe.md"]);
    assertEquals(enResult.code, 0);
    assertStringIncludes(enResult.output, "write-good advisory:");

    const zhResult = await execute(["--locale", "zh-TW", "--files", "docs/writing-locale-probe.md"]);
    assertEquals(zhResult.code, 1);
    assertStringIncludes(zhResult.output, "prh error: 設計系統 => Design System");

    const reqResult = await execute(["--required"]);
    assertEquals(reqResult.code, 2);
    assertStringIncludes(reqResult.output, "docs/writing-locale-probe.md");
    assertStringIncludes(reqResult.output, "must match exactly one locale pattern");
  } finally {
    await Deno.remove(zhProbe);
    await Deno.remove(enProbe);
  }
});

Deno.test("C-04 subcase 1: missing configuration exits 2 with tool failure", async () => {
  const result = await execute(["--locale", "zh-TW", "--files", "non-existent-prose-file.md"]);
  assertEquals(result.code, 2);
  assertStringIncludes(result.output, "Tool failure");
});

Deno.test("C-04 subcase 2: empty cache or missing dependencies exits 2 naming install command", async () => {
  const dictionaryPath = `${scriptDir}/prh.yml`;
  const backupPath = `${scriptDir}/prh.yml.test_bak`;

  await Deno.rename(dictionaryPath, backupPath);
  try {
    const result = await execute(["--locale", "zh-TW", "--files", "README.md"]);
    assertEquals(result.code, 2);
    assertStringIncludes(result.output, "Tool failure");
    assertStringIncludes(result.output, "deno install --config tools/writing/deno.json");
  } finally {
    await Deno.rename(backupPath, dictionaryPath);
  }
});

Deno.test("C-04 subcase 3: unmapped file exits 2 with tool failure", async () => {
  const unmappedDir = `${repoRoot}/docs/unmapped_probe_dir`;
  await Deno.mkdir(unmappedDir, { recursive: true });
  const unmappedFile = `${unmappedDir}/unmapped.md`;
  await Deno.writeTextFile(unmappedFile, "# Unmapped Document\n");

  try {
    const result = await execute(["--required"]);
    assertEquals(result.code, 2);
    assertStringIncludes(result.output, "Tool failure");
    assertStringIncludes(result.output, "docs/unmapped_probe_dir/unmapped.md");
  } finally {
    await Deno.remove(unmappedDir, { recursive: true });
  }
});

Deno.test("C-04 subcase 4: textlint crash or invalid arguments exits 2 with tool failure", async () => {
  const result = await execute(["--locale", "fr-FR", "--files", "README.md"]);
  assertEquals(result.code, 2);
  assertStringIncludes(result.output, "Tool failure");
  assertStringIncludes(result.output, "Usage: --required or --locale <zh-TW|en-US|ja> --files <paths...>");
});

Deno.test("C-05: only advisory findings prints findings and exits 0", async () => {
  const tempFile = await Deno.makeTempFile({ suffix: ".md" });
  const longSentence = "這是一句完全符合名詞規範但是長度超過上限的中文句子，".repeat(8) + "最後在此結束。";
  await Deno.writeTextFile(tempFile, `# 測試標題\n\n${longSentence}\n`);

  try {
    const result = await execute(["--locale", "zh-TW", "--files", tempFile]);
    assertEquals(result.code, 0, "Advisory findings must exit 0");
    assertStringIncludes(result.output, "sentence-length advisory:");
    assertNotIncludes(result.output, "error:");
  } finally {
    await Deno.remove(tempFile);
  }
});

Deno.test("C-08: findings inside the ADR Status History heading are downgraded to advisory (exit 0)", async () => {
  const adrFile = `${repoRoot}/docs/adr/99-test-frozen-history.md`;
  try {
    const content = [
      "# ADR 99: Frozen history test",
      "",
      "## Context",
      "",
      "This is a clean contract paragraph that follows the rules.",
      "",
      "## Status History",
      "",
      "- 2026-01-01: The old repository name (recorded in the history stays as written.",
      "",
    ].join("\n");
    await Deno.writeTextFile(adrFile, content);
    const result = await execute(["--locale", "en-US", "--files", "docs/adr/99-test-frozen-history.md"]);
    assertEquals(result.code, 0, "Frozen history finding must be downgraded to advisory with exit 0");
    assertStringIncludes(result.output, "(frozen history)");
    assertNotIncludes(result.output, "error:");
  } finally {
    await Deno.remove(adrFile).catch(() => {});
  }
});

Deno.test("translations require a unique source and current LF-normalized SHA-256", async () => {
  const translationDir = `${repoRoot}/docs/i18n/zh-TW`;
  const sourcePath = `${repoRoot}/docs/design-systems/writing-freshness-probe.md`;
  const duplicateSourcePath = `${repoRoot}/docs/writing-ambiguity-probe.md`;
  const ambiguousSourcePath = `${repoRoot}/docs/design-systems/writing-ambiguity-probe.md`;
  const translationPath = `${translationDir}/writing-freshness-probe.zh-TW.md`;
  const ambiguousTranslationPath = `${translationDir}/writing-ambiguity-probe.zh-TW.md`;
  const ambiguityCheck = `${scriptDir}/check_ambiguity_probe_test.ts`;
  const sourceLf = "# Freshness Probe\n\nClean source text.\n";
  const sourceCrLf = sourceLf.replaceAll("\n", "\r\n");
  const encoder = new TextEncoder();
  const hash = async (value: string): Promise<string> => {
    const bytes = new Uint8Array(await crypto.subtle.digest("SHA-256", encoder.encode(value.replaceAll("\r\n", "\n"))));
    return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
  };
  const frontMatter = (source?: string, sourceHash?: string) => [
    "---",
    ...(source === undefined ? [] : [`source: ${source}`]),
    "lang: zh-TW",
    ...(sourceHash === undefined ? [] : [`source_sha256: ${sourceHash}`]),
    "---",
    "# 翻譯測試",
    "",
  ].join("\n");
  await Deno.mkdir(translationDir, { recursive: true });
  await Deno.writeTextFile(sourcePath, sourceCrLf);

  try {
    const currentHash = await hash(sourceLf);
    await Deno.writeTextFile(translationPath, frontMatter("docs/design-systems/writing-freshness-probe.md", currentHash));
    const current = await execute(["--required"]);
    assertNotIncludes(current.output, "translation docs/i18n/zh-TW/writing-freshness-probe.zh-TW.md");

    await Deno.writeTextFile(translationPath, frontMatter("docs/design-systems/writing-freshness-probe.md", "0".repeat(64)));
    const stale = await execute(["--required"]);
    assertEquals(stale.code, 1);
    assertStringIncludes(stale.output, `translation docs/i18n/zh-TW/writing-freshness-probe.zh-TW.md is stale; computed source_sha256 ${currentHash}`);

    await Deno.writeTextFile(translationPath, frontMatter("docs/design-systems/writing-freshness-probe.md"));
    const missingKey = await execute(["--required"]);
    assertEquals(missingKey.code, 1);
    assertStringIncludes(missingKey.output, "missing a valid source or source_sha256");

    await Deno.writeTextFile(translationPath, frontMatter("docs/glossary.md", currentHash));
    const wrongSource = await execute(["--required"]);
    assertEquals(wrongSource.code, 1);
    assertStringIncludes(wrongSource.output, "source must be docs/design-systems/writing-freshness-probe.md");

    await Deno.writeTextFile(duplicateSourcePath, "# Duplicate source probe\n");
    await Deno.writeTextFile(ambiguousSourcePath, "# Duplicate source probe\n");
    await Deno.writeTextFile(ambiguousTranslationPath, frontMatter("docs/writing-ambiguity-probe.md", currentHash));
    const checkCode = await Deno.readTextFile(checkScript);
    await Deno.writeTextFile(
      ambiguityCheck,
      checkCode.replace(
        "const pathRules: PathRule[] = [",
        'const pathRules: PathRule[] = [\n  { locale: "en-US", matches: (p) => p === "docs/writing-ambiguity-probe.md" },',
      ),
    );
    const ambiguous = await execute(["--required"], { script: ambiguityCheck });
    assertEquals(ambiguous.code, 1);
    assertStringIncludes(ambiguous.output, "exactly one canonical source candidate, found 2");
    await Deno.remove(ambiguityCheck);
    await Deno.remove(duplicateSourcePath);
    await Deno.remove(ambiguousSourcePath);
    await Deno.remove(ambiguousTranslationPath);

    await Deno.remove(sourcePath);
    const missingSource = await execute(["--required"]);
    assertEquals(missingSource.code, 1);
    assertStringIncludes(missingSource.output, "found 0");
  } finally {
    await Deno.remove(translationPath).catch(() => {});
    await Deno.remove(ambiguousTranslationPath).catch(() => {});
    await Deno.remove(sourcePath).catch(() => {});
    await Deno.remove(duplicateSourcePath).catch(() => {});
    await Deno.remove(ambiguousSourcePath).catch(() => {});
    await Deno.remove(ambiguityCheck).catch(() => {});
    await Deno.remove(translationDir).catch(() => {});
  }
});

Deno.test("ja translations use locale paths, freshness, configuration, and dictionary validation", async () => {
  const jaDir = `${repoRoot}/docs/i18n/ja`;
  let createdDir = false;
  try {
    await Deno.stat(jaDir);
  } catch (error) {
    if (!(error instanceof Deno.errors.NotFound)) throw error;
    await Deno.mkdir(jaDir, { recursive: true });
    createdDir = true;
  }
  const sourcePath = `${repoRoot}/docs/design-systems/writing-ja-probe.md`;
  const jaPath = `${jaDir}/writing-ja-probe.ja.md`;
  const wrongJaPath = `${jaDir}/commands.ja.md`;
  let originalWrongJa: Uint8Array | undefined;
  const tempDir = await Deno.makeTempDir({ dir: scriptDir, prefix: "ja-missing-dictionary-" });
  const encoder = new TextEncoder();
  const hash = async (value: string) => {
    const bytes = new Uint8Array(await crypto.subtle.digest("SHA-256", encoder.encode(value)));
    return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
  };
  const frontMatter = (source: string, digest: string) => `---\nsource: ${source}\nlang: ja\nsource_sha256: ${digest}\n---\n# 翻訳テスト\n\n既定 fallback\n`;
  const originalSource = "# Japanese Probe\n\nA clean source paragraph.\n";
  await Deno.writeTextFile(sourcePath, originalSource);
  try {
    const digest = await hash(originalSource);
    await Deno.writeTextFile(jaPath, frontMatter("docs/design-systems/writing-ja-probe.md", digest));
    const current = await execute(["--required"]);
    assertNotIncludes(current.output, `translation docs/i18n/ja/${jaPath.split("/").at(-1)}`);

    await Deno.writeTextFile(jaPath, frontMatter("docs/design-systems/writing-ja-probe.md", "0".repeat(64)));
    const stale = await execute(["--required"]);
    assertEquals(stale.code, 1);
    assertStringIncludes(stale.output, "writing-ja-probe.ja.md is stale");
    assertStringIncludes(stale.output, digest);

    try {
      originalWrongJa = await Deno.readFile(wrongJaPath);
    } catch (error) {
      if (!(error instanceof Deno.errors.NotFound)) throw error;
    }
    await Deno.writeTextFile(wrongJaPath, frontMatter("docs/glossary.md", digest));
    const wrongSource = await execute(["--required"]);
    assertEquals(wrongSource.code, 1);
    assertStringIncludes(wrongSource.output, "commands.ja.md source must be docs/commands.md");
    const jaRun = await execute(["--locale", "ja", "--files", jaPath]);
    assertEquals(jaRun.code, 0);
    const zhTemp = await Deno.makeTempFile({ suffix: ".md" });
    await Deno.writeTextFile(zhTemp, "既定 fallback\n");
    const zhRun = await execute(["--locale", "zh-TW", "--files", zhTemp]);
    await Deno.remove(zhTemp);
    assertEquals(zhRun.code, 1);
    assertStringIncludes(zhRun.output, "prh error:  fallback => 退回");
    assertNotIncludes(jaRun.output, "prh error:");

    const copied = (await Deno.readTextFile(checkScript)).replace(
      "const repositoryRoot = Deno.realPathSync(`${scriptDirectory}/../..`);",
      `const repositoryRoot = ${JSON.stringify(repoRoot)};`,
    ).replaceAll("`${scriptDirectory}/node_modules`", JSON.stringify(`${scriptDir}/node_modules`));
    await Deno.writeTextFile(`${tempDir}/check.ts`, copied);
    for (const name of ["ja.textlintrc.json", "zh-TW.textlintrc.json", "en-US.textlintrc.json"]) {
      await Deno.copyFile(`${scriptDir}/${name}`, `${tempDir}/${name}`);
    }
    const missing = await execute(["--locale", "ja", "--files", jaPath], { script: `${tempDir}/check.ts` });
    assertEquals(missing.code, 2);
    assertStringIncludes(missing.output, "prh.ja.yml");

    const usage = await execute(["--locale", "fr-FR", "--files", jaPath]);
    assertStringIncludes(usage.output, "<zh-TW|en-US|ja>");
  } finally {
    await Deno.remove(jaPath).catch(() => {});
    if (originalWrongJa !== undefined) {
      await Deno.writeFile(wrongJaPath, originalWrongJa);
    } else {
      await Deno.remove(wrongJaPath).catch(() => {});
    }
    await Deno.remove(sourcePath).catch(() => {});
    await Deno.remove(tempDir, { recursive: true }).catch(() => {});
    if (createdDir) await Deno.remove(jaDir, { recursive: true }).catch(() => {});
  }
});

Deno.test("C-10: rerun on unchanged files produces identical findings and exit code", async () => {
  const run1 = await execute(["--locale", "zh-TW", "--files", "README.md"]);
  const run2 = await execute(["--locale", "zh-TW", "--files", "README.md"]);
  assertEquals(run1.code, run2.code, "Exit codes must be identical across reruns");
  assertEquals(run1.output, run2.output, "Output must be identical across reruns");
});

Deno.test("C-12: prh specs entry that no longer matches rule causes tool failure (exit 2)", async () => {
  const prhPath = `${scriptDir}/prh.yml`;
  const original = await Deno.readTextFile(prhPath);
  const brokenSpec = `${original}\n  - expected: 正確詞\n    specs:\n      - from: 錯誤詞\n        to: 不符合的預期\n`;

  await Deno.writeTextFile(prhPath, brokenSpec);
  try {
    const result = await execute(["--locale", "zh-TW", "--files", "README.md"]);
    assertEquals(result.code, 2, "Invalid prh specs must cause exit 2 tool failure");
    assertStringIncludes(result.output, "Tool failure");
  } finally {
    await Deno.writeTextFile(prhPath, original);
  }
});

Deno.test("unmapped file under a new docs subdirectory fails with exit 2 in required mode", async () => {
  const newSubdir = `${repoRoot}/docs/new_experimental_section`;
  await Deno.mkdir(newSubdir, { recursive: true });
  const newFile = `${newSubdir}/index.md`;
  await Deno.writeTextFile(newFile, "# Experimental Section\n");

  try {
    const result = await execute(["--required"]);
    assertEquals(result.code, 2);
    assertStringIncludes(result.output, "Tool failure");
    assertStringIncludes(result.output, "docs/new_experimental_section/index.md");
  } finally {
    await Deno.remove(newSubdir, { recursive: true });
  }
});

Deno.test("overlapping pattern fails with exit 2 in required mode", async () => {
  const checkCode = await Deno.readTextFile(checkScript);
  const overlapCode = checkCode.replace(
    'const pathRules: PathRule[] = [',
    'const pathRules: PathRule[] = [\n  { locale: "en-US", matches: (p) => p === "docs/architecture.md" },',
  );
  const tempCheck = `${scriptDir}/check_overlap_test.ts`;
  await Deno.writeTextFile(tempCheck, overlapCode);

  try {
    const result = await execute(["--required"], { script: tempCheck });
    assertEquals(result.code, 2);
    assertStringIncludes(result.output, "Tool failure");
    assertStringIncludes(result.output, "docs/architecture.md");
  } finally {
    await Deno.remove(tempCheck);
  }
});

Deno.test("fence-mode execution runs cleanly from repo root and outside tools/writing without network permission and leaves deno.lock unchanged", async () => {
  const cleanDoc = `${repoRoot}/docs/probe-clean-doc.md`;
  await Deno.writeTextFile(cleanDoc, "# Clean Test Document\n\n這是一份完全沒有任何違規用詞與長度問題的測試文件。\n");

  try {
    // 1. Run from repository root
    const rootRun = await execute(
      ["--locale", "zh-TW", "--files", "docs/probe-clean-doc.md"],
      {
        cwd: repoRoot,
        permissions: ["--allow-read", "--allow-env", "--allow-sys"],
      },
    );
    assertEquals(rootRun.code, 0, "Fence mode from repository root must exit 0 on clean file");

    // 2. Run from directory outside tools/writing (e.g. tests/)
    const outsideCwd = `${repoRoot}/tests`;
    const outsideRun = await execute(
      ["--locale", "zh-TW", "--files", cleanDoc],
      {
        cwd: outsideCwd,
        permissions: ["--allow-read", "--allow-env", "--allow-sys"],
      },
    );
    assertEquals(outsideRun.code, 0, "Fence mode from outside tools/writing must exit 0 on clean file");

    // 3. Confirm permissions: verify that neither command was granted --allow-net or --allow-write
    // (explicit perms array in execute above has only --allow-read, --allow-env, --allow-sys)

    // 4. Verify deno.lock is unchanged
    const diffCmd = new Deno.Command("git", {
      args: ["diff", "--exit-code", "tools/writing/deno.lock"],
      cwd: repoRoot,
    });
    const diffRes = await diffCmd.output();
    assertEquals(diffRes.code, 0, "tools/writing/deno.lock must remain clean and unmodified");
  } finally {
    await Deno.remove(cleanDoc);
  }
});
