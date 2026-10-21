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
    assertEquals(reqResult.code, 1);
    assertStringIncludes(reqResult.output, "docs/writing-locale-probe.md");
    assertStringIncludes(reqResult.output, "docs/templates/generated-project/writing-locale-probe.md");
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
  assertStringIncludes(result.output, "Usage: --required or --locale <zh-TW|en-US> --files <paths...>");
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

Deno.test("C-08: findings inside ADR 狀態沿革 are downgraded to advisory and labeled frozen history (exit 0)", async () => {
  const adrFile = `${repoRoot}/docs/adr/99-test-frozen-history.md`;
  const content = [
    "# ADR 99: 測試凍結歷史",
    "",
    "## 背景",
    "",
    "這裡是一段乾淨的契約內容，完全符合規範。",
    "",
    "## 狀態沿革",
    "",
    "- 2026-01-01：舊版的倉庫名稱紀錄在歷史沿革中。",
    "",
  ].join("\n");
  await Deno.writeTextFile(adrFile, content);

  try {
    const result = await execute(["--locale", "zh-TW", "--files", "docs/adr/99-test-frozen-history.md"]);
    assertEquals(result.code, 0, "Frozen history finding must be downgraded to advisory with exit 0");
    assertStringIncludes(result.output, "prh advisory: 倉庫 => 儲存庫 (frozen history)");
    assertNotIncludes(result.output, "error:");
  } finally {
    await Deno.remove(adrFile);
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
    '{ locale: "zh-TW", matches: (p) => /^docs\\/[^/]+\\.md$/.test(p) },',
    '{ locale: "zh-TW", matches: (p) => /^docs\\/[^/]+\\.md$/.test(p) },\n  { locale: "en-US", matches: (p) => p === "docs/architecture.md" },',
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
