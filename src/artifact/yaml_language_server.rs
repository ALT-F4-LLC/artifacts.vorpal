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
pub struct YamlLanguageServer<'a> {
    node: Option<&'a str>,
}

impl<'a> YamlLanguageServer<'a> {
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

        let name = "yaml-language-server";
        let source_version = "1.23.0";

        let source_path =
            format!("https://registry.npmjs.org/{name}/-/{name}-{source_version}.tgz");

        // Runtime dependency closure (production only) of yaml-language-server@1.23.0, resolved flat
        // (one version per package). The published tarball ships unbundled out/ that `require`s these
        // at runtime, and vorpal build steps run offline, so each dep is vendored here as a
        // pre-fetched source tarball and copied into a flat node_modules during the build step.
        // Pinned from registry.npmjs.org (the package authors' official publish channel). Source keys
        // are "yls-"-prefixed because vorpal locks sources by key globally, and bare npm names (e.g.
        // "typescript", "semver") collide with other artifacts' sources.
        //
        // vscode-languageserver-types is pinned to 3.17.5 (not the higher 3.18.0 that satisfies the
        // caret consumers) because vscode-languageserver-protocol@3.17.5 requires it EXACTLY; 3.17.5
        // is the only version that satisfies all three consumers under flat dedup.
        let sources = vec![
            ArtifactSource::new(name, &source_path).build(),
            ArtifactSource::new("yls-vscode-l10n", "https://registry.npmjs.org/@vscode/l10n/-/l10n-0.0.18.tgz").build(),
            ArtifactSource::new("yls-ajv", "https://registry.npmjs.org/ajv/-/ajv-8.20.0.tgz").build(),
            ArtifactSource::new("yls-ajv-draft-04", "https://registry.npmjs.org/ajv-draft-04/-/ajv-draft-04-1.0.0.tgz").build(),
            ArtifactSource::new("yls-ajv-i18n", "https://registry.npmjs.org/ajv-i18n/-/ajv-i18n-4.2.0.tgz").build(),
            ArtifactSource::new("yls-fast-deep-equal", "https://registry.npmjs.org/fast-deep-equal/-/fast-deep-equal-3.1.3.tgz").build(),
            ArtifactSource::new("yls-fast-uri", "https://registry.npmjs.org/fast-uri/-/fast-uri-3.1.2.tgz").build(),
            ArtifactSource::new("yls-json-schema-traverse", "https://registry.npmjs.org/json-schema-traverse/-/json-schema-traverse-1.0.0.tgz").build(),
            ArtifactSource::new("yls-jsonc-parser", "https://registry.npmjs.org/jsonc-parser/-/jsonc-parser-3.3.1.tgz").build(),
            ArtifactSource::new("yls-prettier", "https://registry.npmjs.org/prettier/-/prettier-3.8.4.tgz").build(),
            ArtifactSource::new("yls-request-light", "https://registry.npmjs.org/request-light/-/request-light-0.5.8.tgz").build(),
            ArtifactSource::new("yls-require-from-string", "https://registry.npmjs.org/require-from-string/-/require-from-string-2.0.2.tgz").build(),
            ArtifactSource::new("yls-vscode-json-languageservice", "https://registry.npmjs.org/vscode-json-languageservice/-/vscode-json-languageservice-4.1.8.tgz").build(),
            ArtifactSource::new("yls-vscode-jsonrpc", "https://registry.npmjs.org/vscode-jsonrpc/-/vscode-jsonrpc-8.2.0.tgz").build(),
            ArtifactSource::new("yls-vscode-languageserver", "https://registry.npmjs.org/vscode-languageserver/-/vscode-languageserver-9.0.1.tgz").build(),
            ArtifactSource::new("yls-vscode-languageserver-protocol", "https://registry.npmjs.org/vscode-languageserver-protocol/-/vscode-languageserver-protocol-3.17.5.tgz").build(),
            ArtifactSource::new("yls-vscode-languageserver-textdocument", "https://registry.npmjs.org/vscode-languageserver-textdocument/-/vscode-languageserver-textdocument-1.0.13.tgz").build(),
            ArtifactSource::new("yls-vscode-languageserver-types", "https://registry.npmjs.org/vscode-languageserver-types/-/vscode-languageserver-types-3.17.5.tgz").build(),
            ArtifactSource::new("yls-vscode-nls", "https://registry.npmjs.org/vscode-nls/-/vscode-nls-5.2.0.tgz").build(),
            ArtifactSource::new("yls-vscode-uri", "https://registry.npmjs.org/vscode-uri/-/vscode-uri-3.1.0.tgz").build(),
            ArtifactSource::new("yls-yaml", "https://registry.npmjs.org/yaml/-/yaml-2.8.3.tgz").build(),
        ];

        let env_node = get_env_key(&node.to_string());

        let pkg_dir = format!("$VORPAL_OUTPUT/lib/node_modules/{name}");

        // Copy the package, then assemble a flat node_modules from the vendored dep sources. Node's
        // module resolution walks up from the bin script, so a single flat node_modules at the
        // package root satisfies the server. The closure is conflict-free (no two deps need
        // incompatible majors), so flat resolution is correct.
        let step_script = formatdoc! {"
            mkdir -pv \"{pkg_dir}\" \"$VORPAL_OUTPUT/bin\"

            cp -Rv ./source/{name}/package/* \"{pkg_dir}/.\"

            NM=\"{pkg_dir}/node_modules\"
            mkdir -pv \"$NM\" \"$NM/@vscode\"

            cp -Rv ./source/yls-vscode-l10n/package \"$NM/@vscode/l10n\"
            cp -Rv ./source/yls-ajv/package \"$NM/ajv\"
            cp -Rv ./source/yls-ajv-draft-04/package \"$NM/ajv-draft-04\"
            cp -Rv ./source/yls-ajv-i18n/package \"$NM/ajv-i18n\"
            cp -Rv ./source/yls-fast-deep-equal/package \"$NM/fast-deep-equal\"
            cp -Rv ./source/yls-fast-uri/package \"$NM/fast-uri\"
            cp -Rv ./source/yls-json-schema-traverse/package \"$NM/json-schema-traverse\"
            cp -Rv ./source/yls-jsonc-parser/package \"$NM/jsonc-parser\"
            cp -Rv ./source/yls-prettier/package \"$NM/prettier\"
            cp -Rv ./source/yls-request-light/package \"$NM/request-light\"
            cp -Rv ./source/yls-require-from-string/package \"$NM/require-from-string\"
            cp -Rv ./source/yls-vscode-json-languageservice/package \"$NM/vscode-json-languageservice\"
            cp -Rv ./source/yls-vscode-jsonrpc/package \"$NM/vscode-jsonrpc\"
            cp -Rv ./source/yls-vscode-languageserver/package \"$NM/vscode-languageserver\"
            cp -Rv ./source/yls-vscode-languageserver-protocol/package \"$NM/vscode-languageserver-protocol\"
            cp -Rv ./source/yls-vscode-languageserver-textdocument/package \"$NM/vscode-languageserver-textdocument\"
            cp -Rv ./source/yls-vscode-languageserver-types/package \"$NM/vscode-languageserver-types\"
            cp -Rv ./source/yls-vscode-nls/package \"$NM/vscode-nls\"
            cp -Rv ./source/yls-vscode-uri/package \"$NM/vscode-uri\"
            cp -Rv ./source/yls-yaml/package \"$NM/yaml\"

            cat << EOF > \"$VORPAL_OUTPUT/bin/{name}\"
            #!/bin/sh
            exec {env_node}/bin/node \"{pkg_dir}/bin/{name}\" \"\\$@\"
            EOF

            chmod +x \"$VORPAL_OUTPUT/bin/{name}\"",
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
            {env_node}/bin/node \"$selftest_dir/driver.mjs\" \"$VORPAL_OUTPUT/bin/{name}\" \\
                || (echo 'ERROR: yaml-language-server self-test failed (LSP did not start / resolve deps)' && exit 1)",
            SELFTEST_DRIVER_JS = SELFTEST_DRIVER_JS,
            env_node = env_node,
            name = name,
        };

        let step_script = format!("{step_script}\n{selftest_setup}");

        let steps =
            vec![step::shell(context, vec![node.to_string()], vec![], step_script, vec![]).await?];

        let systems = vec![Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux];

        Artifact::new(name, steps, systems)
            .with_aliases(vec![format!("{name}:{source_version}")])
            .with_sources(sources)
            .build(context)
            .await
    }
}
