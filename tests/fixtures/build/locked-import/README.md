# Locked-import dependency fixture

This fixture declares `preact` with the `^10.29.8` range in `package.json`.
Its `scripts/main.ts` imports the `preact/hooks` subpath and uses its `useState`
export. The native build resolves the subpath from packages installed in
`node_modules`.

The `deno.lock` file records the historical resolved version, `preact@10.29.8`,
as fixture data. The native build does not use it for dependency resolution.
