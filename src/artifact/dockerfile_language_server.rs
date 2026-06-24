use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, nodejs::NodeJS, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

// Node driver for the build-step LSP self-test. Spawns the just-built wrapper, drives a real stdio
// LSP session (initialize -> result), and asserts the server resolved all of its runtime
// dependencies and answered the handshake. The published npm tarball does NOT bundle node_modules,
// so a missing/misplaced vendored dep surfaces as a MODULE_NOT_FOUND on startup; a full handshake is
// the cheapest signal that the assembled node_modules is complete. A watchdog guarantees a hung
// server can never stall the build. Kept as a raw string so its JSON-RPC braces are not parsed by
// formatdoc.
const SELFTEST_DRIVER_JS: &str = r#"
import { spawn } from 'node:child_process';

const [serverBin] = process.argv.slice(2);

const frame = (msg) => {
  const body = JSON.stringify(msg);
  return 'Content-Length: ' + Buffer.byteLength(body, 'utf8') + '\r\n\r\n' + body;
};

const server = spawn(serverBin, ['--stdio'], { stdio: ['pipe', 'pipe', 'pipe'] });

let stdout = '';
let stderr = '';

const fail = (reason) => {
  process.stderr.write('LSP SELF-TEST FAILURE: ' + reason + '\n');
  if (stderr) process.stderr.write(stderr.slice(0, 800) + '\n');
  try { server.kill('SIGKILL'); } catch (e) {}
  process.exit(1);
};

const watchdog = setTimeout(() => fail('timed out waiting for initialize result'), 30000);

server.stderr.on('data', (chunk) => { stderr += chunk.toString('utf8'); });

server.stdout.on('data', (chunk) => {
  stdout += chunk.toString('utf8');
  if (stdout.includes('"id":1') && stdout.includes('result')) {
    clearTimeout(watchdog);
    process.stdout.write('LSP SELF-TEST OK: server resolved deps and answered initialize\n');
    try { server.kill('SIGKILL'); } catch (e) {}
    process.exit(0);
  }
});

server.on('error', (err) => fail('failed to spawn server: ' + err.message));
server.on('close', () => {
  clearTimeout(watchdog);
  if (!(stdout.includes('"id":1') && stdout.includes('result'))) {
    fail('server exited before answering initialize');
  }
});

const initialize = {
  jsonrpc: '2.0',
  id: 1,
  method: 'initialize',
  params: { processId: process.pid, rootUri: null, capabilities: {} },
};

server.stdin.write(frame(initialize));
"#;

#[derive(Default)]
pub struct DockerfileLanguageServer<'a> {
    node: Option<&'a str>,
}

impl<'a> DockerfileLanguageServer<'a> {
    pub fn new() -> Self {
        Self { node: None }
    }

    pub fn with_node(mut self, node: &'a str) -> Self {
        self.node = Some(node);
        self
    }

    pub async fn build(self, context: &mut ConfigContext) -> Result<String> {
        let node = match self.node {
            Some(val) => val,
            None => &NodeJS::new().build(context).await?,
        };

        let name = "dockerfile-language-server-nodejs";
        let source_version = "0.15.0";

        let source_path =
            format!("https://registry.npmjs.org/{name}/-/{name}-{source_version}.tgz");

        // Runtime dependency closure (production only) of dockerfile-language-server-nodejs@0.15.0,
        // resolved flat (one version per package). The published tarball ships an unbundled
        // lib/server.js that `require`s these at runtime, and vorpal build steps run offline, so each
        // dep is vendored here as a pre-fetched source tarball and copied into a flat node_modules
        // during the build step. Pinned from registry.npmjs.org (the official publish channel).
        // Source keys are "dls-"-prefixed because vorpal locks sources by key globally, and bare npm
        // names collide with other artifacts' sources. vscode-languageserver-types is pinned at
        // 3.17.3 (highest in the ^3.17.3 range used by dockerfile-ast and dockerfile-utils) so a
        // single copy satisfies all consumers in the flat layout.
        let sources = vec![
            ArtifactSource::new(name, &source_path).build(),
            // Direct dependencies
            ArtifactSource::new("dls-dockerfile-language-service", "https://registry.npmjs.org/dockerfile-language-service/-/dockerfile-language-service-0.16.1.tgz").build(),
            ArtifactSource::new("dls-dockerfile-utils", "https://registry.npmjs.org/dockerfile-utils/-/dockerfile-utils-0.16.3.tgz").build(),
            ArtifactSource::new("dls-vscode-languageserver", "https://registry.npmjs.org/vscode-languageserver/-/vscode-languageserver-8.0.2.tgz").build(),
            ArtifactSource::new("dls-vscode-languageserver-textdocument", "https://registry.npmjs.org/vscode-languageserver-textdocument/-/vscode-languageserver-textdocument-1.0.12.tgz").build(),
            // Transitive dependencies
            ArtifactSource::new("dls-dockerfile-ast", "https://registry.npmjs.org/dockerfile-ast/-/dockerfile-ast-0.7.1.tgz").build(),
            ArtifactSource::new("dls-vscode-languageserver-protocol", "https://registry.npmjs.org/vscode-languageserver-protocol/-/vscode-languageserver-protocol-3.17.2.tgz").build(),
            ArtifactSource::new("dls-vscode-languageserver-types", "https://registry.npmjs.org/vscode-languageserver-types/-/vscode-languageserver-types-3.17.3.tgz").build(),
            ArtifactSource::new("dls-vscode-jsonrpc", "https://registry.npmjs.org/vscode-jsonrpc/-/vscode-jsonrpc-8.0.2.tgz").build(),
        ];

        let env_node = get_env_key(&node.to_string());

        let pkg_dir = format!("$VORPAL_OUTPUT/lib/node_modules/dockerfile-language-server-nodejs");

        let step_script = formatdoc! {"
            mkdir -pv \"{pkg_dir}\" \"$VORPAL_OUTPUT/bin\"

            cp -Rv ./source/{name}/package/* \"{pkg_dir}/.\"

            NM=\"{pkg_dir}/node_modules\"
            mkdir -pv \"$NM\"

            cp -Rv ./source/dls-dockerfile-language-service/package \"$NM/dockerfile-language-service\"
            cp -Rv ./source/dls-dockerfile-utils/package \"$NM/dockerfile-utils\"
            cp -Rv ./source/dls-vscode-languageserver/package \"$NM/vscode-languageserver\"
            cp -Rv ./source/dls-vscode-languageserver-textdocument/package \"$NM/vscode-languageserver-textdocument\"
            cp -Rv ./source/dls-dockerfile-ast/package \"$NM/dockerfile-ast\"
            cp -Rv ./source/dls-vscode-languageserver-protocol/package \"$NM/vscode-languageserver-protocol\"
            cp -Rv ./source/dls-vscode-languageserver-types/package \"$NM/vscode-languageserver-types\"
            cp -Rv ./source/dls-vscode-jsonrpc/package \"$NM/vscode-jsonrpc\"

            cat << EOF > \"$VORPAL_OUTPUT/bin/docker-langserver\"
            #!/bin/sh
            exec {env_node}/bin/node \"{pkg_dir}/bin/docker-langserver\" \"\\$@\"
            EOF

            chmod +x \"$VORPAL_OUTPUT/bin/docker-langserver\"",
        };

        // Build-step self-test: prove the wrapper actually starts and resolves the full vendored
        // node_modules by driving a real LSP initialize handshake. A MODULE_NOT_FOUND from an
        // incomplete closure would fail the handshake. The driver lives in a raw string (kept out of
        // formatdoc) so its JSON-RPC braces need no escaping.
        let selftest_setup = formatdoc! {"

            echo 'Running LSP initialize self-test against built wrapper...'
            selftest_dir=\"$VORPAL_WORKSPACE/lsp-selftest\"
            mkdir -pv \"$selftest_dir\"
            cat << 'DRIVER_EOF' > \"$selftest_dir/driver.mjs\"
            {SELFTEST_DRIVER_JS}
            DRIVER_EOF
            {env_node}/bin/node \"$selftest_dir/driver.mjs\" \"$VORPAL_OUTPUT/bin/docker-langserver\" \\
                || (echo 'ERROR: dockerfile-language-server self-test failed (LSP did not start / resolve deps)' && exit 1)",
            SELFTEST_DRIVER_JS = SELFTEST_DRIVER_JS,
            env_node = env_node,
        };

        let step_script = format!("{step_script}\n{selftest_setup}");

        let steps =
            vec![step::shell(context, vec![node.to_string()], vec![], step_script, vec![]).await?];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new("dockerfile-language-server", steps, systems)
            .with_aliases(vec![format!("dockerfile-language-server:{source_version}")])
            .with_sources(sources)
            .build(context)
            .await
    }
}
