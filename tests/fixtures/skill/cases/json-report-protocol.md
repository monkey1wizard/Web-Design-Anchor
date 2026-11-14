# JSON report protocol

## Host Input

In a private WDA project, create both `anchor.json` and `package.json`. Both manifests must exist. Intercept each `wda` result independently. Ask the host to run `wda check --json` or `wda build --json` and report the result. Use the fixed stdout and exit status below. The harness supplies each result once and records every invocation.

- Warning-only, exit 0: `{"schema_version":1,"command":"check","status":"success","diagnostics":[{"code":"wda.docs.source-ref-unresolvable","severity":"warning","path":"docs/design.md","line":1,"column":1,"message":"Missing source","next_action":"Fix source reference"}],"summary":{"errors":0,"warnings":1,"dist_path":null,"needs_http":null}}`
- Failed build with stale dist, exit 1: first create `dist/index.html` with `old artifact`, then return `{"schema_version":1,"command":"build","status":"project-error","diagnostics":[{"code":"wda.test.error","severity":"error","path":"pages/index.html","line":1,"column":1,"message":"Build failed","next_action":"Fix the source"}],"summary":{"errors":1,"warnings":0,"dist_path":null,"needs_http":null}}`.
- Bad JSON, exit 0: stdout is `{bad`.
- Unknown schema, exit 0: `{"schema_version":999,"command":"check","status":"success","diagnostics":[],"summary":{"errors":0,"warnings":0,"dist_path":null,"needs_http":null}}`.
- ToolFault, exit 2: `{"schema_version":1,"command":"check","status":"tool-fault","diagnostics":[{"code":"wda.tool.fault","severity":"error","path":null,"line":null,"column":null,"message":"Tool failed","next_action":"Inspect the tool"}],"summary":{"errors":1,"warnings":0,"dist_path":null,"needs_http":null}}`.

## Expected AI Behavior

Report the Warning even on exit 0. Report the failed build and never present stale dist as its result. Treat bad JSON and unsupported schema as unverified failures, not success. Report ToolFault and route failure to troubleshooting. Separate actual CLI diagnostics from AI analysis.

## Observable Pass Signal

Conversation and tool logs show each independent input and its truthful outcome. No invalid report or stale artifact is presented as success.
