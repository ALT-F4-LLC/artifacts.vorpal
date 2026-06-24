use crate::artifact::typescript::Typescript;
use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, nodejs::NodeJS, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

// Node driver for the build-step LSP self-test. Spawns the just-built wrapper, drives a real stdio
// LSP session (initialize -> initialized -> didOpen of a file with a deliberate type error), and
// asserts BOTH that tsserver resolved typescript ("Using Typescript", and no "Could not find a
// valid TypeScript installation") AND that it served a diagnostic ("is not assignable"). On success
// it sends shutdown+exit so the server terminates and the pipe closes deterministically; a watchdog
// guarantees a hung server can never stall the build. Kept as a raw string so its JSON-RPC braces
// are not parsed by formatdoc.
const SELFTEST_DRIVER_JS: &str = r#"
import { spawn } from 'node:child_process';
import { readFileSync } from 'node:fs';

const [serverBin, tsFile] = process.argv.slice(2);
const uri = 'file://' + tsFile;
const text = readFileSync(tsFile, 'utf8');

const frame = (msg) => {
  const body = JSON.stringify(msg);
  return 'Content-Length: ' + Buffer.byteLength(body, 'utf8') + '\r\n\r\n' + body;
};

const server = spawn(serverBin, ['--stdio'], { stdio: ['pipe', 'pipe', 'pipe'] });

let stdout = '';
let combined = '';
let sawDiagnostic = false;
let shuttingDown = false;

const fail = (reason) => {
  process.stderr.write('LSP SELF-TEST FAILURE: ' + reason + '\n');
  try { server.kill('SIGKILL'); } catch (e) {}
  process.exit(1);
};

const watchdog = setTimeout(() => fail('timed out waiting for publishDiagnostics'), 30000);

// tsserver's resolution log ("Using Typescript ...") may surface on stderr or as an LSP
// window/logMessage on stdout, so we assert resolution against the combined stream and keep the
// diagnostic assertion on the LSP protocol stdout. Echo both to our stderr for build-log visibility.
server.stderr.on('data', (chunk) => {
  const s = chunk.toString('utf8');
  combined += s;
  process.stderr.write(s);
});

server.stdout.on('data', (chunk) => {
  const s = chunk.toString('utf8');
  stdout += s;
  combined += s;
  if (!sawDiagnostic && stdout.includes('publishDiagnostics') && stdout.includes('is not assignable')) {
    sawDiagnostic = true;
    if (!shuttingDown) {
      shuttingDown = true;
      server.stdin.write(frame({ jsonrpc: '2.0', id: 2, method: 'shutdown' }));
      server.stdin.write(frame({ jsonrpc: '2.0', method: 'exit' }));
      server.stdin.end();
    }
  }
});

server.on('error', (err) => fail('failed to spawn server: ' + err.message));

server.on('close', () => {
  clearTimeout(watchdog);
  if (combined.includes('Could not find a valid TypeScript installation')) {
    fail('tsserver could not resolve a valid TypeScript installation');
  }
  if (!combined.includes('Using Typescript')) {
    fail('did not observe tsserver resolving typescript (no "Using Typescript")');
  }
  if (!sawDiagnostic) {
    fail('did not observe a publishDiagnostics with the expected type error');
  }
  process.stdout.write('LSP SELF-TEST OK: resolved typescript and served diagnostics\n');
  process.exit(0);
});

const initialize = {
  jsonrpc: '2.0',
  id: 1,
  method: 'initialize',
  params: {
    processId: process.pid,
    rootUri: 'file://' + process.cwd(),
    capabilities: { textDocument: { publishDiagnostics: {} } },
  },
};
const initialized = { jsonrpc: '2.0', method: 'initialized', params: {} };
const didOpen = {
  jsonrpc: '2.0',
  method: 'textDocument/didOpen',
  params: {
    textDocument: { uri, languageId: 'typescript', version: 1, text },
  },
};

server.stdin.write(frame(initialize));
server.stdin.write(frame(initialized));
server.stdin.write(frame(didOpen));
"#;

#[derive(Default)]
pub struct TypescriptLanguageServer<'a> {
    node: Option<&'a str>,
    typescript: Option<&'a str>,
}

impl<'a> TypescriptLanguageServer<'a> {
    pub fn new() -> Self {
        Self {
            node: None,
            typescript: None,
        }
    }

    pub fn with_node(mut self, node: &'a str) -> Self {
        self.node = Some(node);
        self
    }

    pub fn with_typescript(mut self, typescript: &'a str) -> Self {
        self.typescript = Some(typescript);
        self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let node = match self.node {
            Some(val) => val,
            None => &NodeJS::new().build(context).await?,
        };

        let typescript = match self.typescript {
            Some(val) => val,
            None => &Typescript::new().with_node(node).build(context).await?,
        };

        let name = "typescript-language-server";
        let source_version = "5.3.0";

        let source_path = format!(
            "https://registry.npmjs.org/typescript-language-server/-/typescript-language-server-{source_version}.tgz"
        );

        let source = ArtifactSource::new(name, &source_path).build();

        let env_node = get_env_key(&node.to_string());
        let env_typescript = get_env_key(&typescript.to_string());

        let step_script = formatdoc! {"
            mkdir -pv \"$VORPAL_OUTPUT/bin\" \"$VORPAL_OUTPUT/lib/node_modules/typescript-language-server\"

            pushd ./source/{name}/package

            cp -Rv * \"$VORPAL_OUTPUT/lib/node_modules/typescript-language-server/.\"

            popd

            # typescript-language-server has no --tsserver-path flag; it locates tsserver via
            # require.resolve('typescript'), so link the typescript artifact into its node_modules.
            mkdir -pv \"$VORPAL_OUTPUT/lib/node_modules/typescript-language-server/node_modules\"
            ln -svf {env_typescript}/lib/node_modules/typescript \"$VORPAL_OUTPUT/lib/node_modules/typescript-language-server/node_modules/typescript\"

            cat << EOF > \"$VORPAL_OUTPUT/bin/typescript-language-server\"
            #!/bin/sh
            exec {env_node}/bin/node \"$VORPAL_OUTPUT/lib/node_modules/typescript-language-server/lib/cli.mjs\" \"\\$@\"
            EOF

            chmod +x \"$VORPAL_OUTPUT/bin/typescript-language-server\""
        };

        // Build-step self-test: prove the wrapper both RESOLVES typescript (tsserver) and SERVES a
        // real LSP session. This catches the class of regression where the artifact builds fine but
        // the language server dies at startup or can't locate typescript (broken symlink, bogus
        // --tsserver-path flag). A shallow --version check would exit before resolution and miss it,
        // so we drive a full stdio session against a file with a deliberate type error and assert
        // both halves of the contract. The driver lives in a raw string (kept out of formatdoc) so
        // its JSON-RPC braces need no escaping.
        let selftest_setup = formatdoc! {"

            echo 'Running LSP resolve+serve self-test against built wrapper...'
            selftest_dir=\"$VORPAL_WORKSPACE/lsp-selftest\"
            mkdir -pv \"$selftest_dir\"
            printf 'const n: number = \"x\";\\n' > \"$selftest_dir/bad.ts\"
            cat << 'DRIVER_EOF' > \"$selftest_dir/driver.mjs\"
            {SELFTEST_DRIVER_JS}
            DRIVER_EOF
            {env_node}/bin/node \"$selftest_dir/driver.mjs\" \"$VORPAL_OUTPUT/bin/typescript-language-server\" \"$selftest_dir/bad.ts\" \\
                || (echo 'ERROR: typescript-language-server self-test failed (LSP did not resolve typescript and/or serve diagnostics)' && exit 1)",
            SELFTEST_DRIVER_JS = SELFTEST_DRIVER_JS,
            env_node = env_node,
        };

        let step_script = format!("{step_script}\n{selftest_setup}");

        let steps = vec![
            step::shell(
                context,
                vec![node.to_string(), typescript.to_string()],
                vec![],
                step_script,
                vec![],
            )
            .await?,
        ];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{source_version}")])
            .with_sources(vec![source])
            .build(context)
            .await
    }
}
