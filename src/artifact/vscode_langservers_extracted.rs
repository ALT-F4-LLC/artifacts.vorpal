use anyhow::Result;
use indoc::formatdoc;
use vorpal_sdk::{
    api::artifact::ArtifactSystem::{Aarch64Darwin, Aarch64Linux, X8664Darwin, X8664Linux},
    artifact::{get_env_key, nodejs::NodeJS, step, Artifact, ArtifactSource},
    context::ConfigContext,
};

// Node driver for the build-step LSP self-test. Spawns one of the just-built wrappers, drives a
// real stdio LSP session (initialize -> result), and asserts the server resolved all of its runtime
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
pub struct VscodeLangserversExtracted<'a> {
    node: Option<&'a str>,
}

impl<'a> VscodeLangserversExtracted<'a> {
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

        let name = "vscode-langservers-extracted";
        let source_version = "4.10.0";

        let source_path =
            format!("https://registry.npmjs.org/{name}/-/{name}-{source_version}.tgz");

        // Runtime dependency closure (production only) of vscode-langservers-extracted@4.10.0. The
        // published tarball ships unbundled lib/ that `require`s these at runtime, and vorpal build
        // steps run offline, so each dep is vendored here as a pre-fetched source tarball and copied
        // into a flat node_modules during the build step. Pinned from registry.npmjs.org (the
        // package authors' official publish channel). Source keys are "vlse-"-prefixed because vorpal
        // locks sources by key globally, and bare npm names (e.g. "typescript") collide with other
        // artifacts' sources.
        let sources = vec![
            ArtifactSource::new(name, &source_path).build(),
            ArtifactSource::new("vlse-vscode-l10n", "https://registry.npmjs.org/@vscode/l10n/-/l10n-0.0.18.tgz").build(),
            ArtifactSource::new("vlse-boolbase", "https://registry.npmjs.org/boolbase/-/boolbase-1.0.0.tgz").build(),
            ArtifactSource::new("vlse-core-js", "https://registry.npmjs.org/core-js/-/core-js-3.49.0.tgz").build(),
            ArtifactSource::new("vlse-css-select", "https://registry.npmjs.org/css-select/-/css-select-5.2.2.tgz").build(),
            ArtifactSource::new("vlse-css-what", "https://registry.npmjs.org/css-what/-/css-what-6.2.2.tgz").build(),
            ArtifactSource::new("vlse-dom-serializer", "https://registry.npmjs.org/dom-serializer/-/dom-serializer-2.0.0.tgz").build(),
            ArtifactSource::new("vlse-domelementtype", "https://registry.npmjs.org/domelementtype/-/domelementtype-2.3.0.tgz").build(),
            ArtifactSource::new("vlse-domhandler", "https://registry.npmjs.org/domhandler/-/domhandler-5.0.3.tgz").build(),
            ArtifactSource::new("vlse-domutils", "https://registry.npmjs.org/domutils/-/domutils-3.2.2.tgz").build(),
            ArtifactSource::new("vlse-entities", "https://registry.npmjs.org/entities/-/entities-4.5.0.tgz").build(),
            ArtifactSource::new("vlse-he", "https://registry.npmjs.org/he/-/he-1.2.0.tgz").build(),
            ArtifactSource::new("vlse-jsonc-parser", "https://registry.npmjs.org/jsonc-parser/-/jsonc-parser-3.3.1.tgz").build(),
            ArtifactSource::new("vlse-node-html-parser", "https://registry.npmjs.org/node-html-parser/-/node-html-parser-6.1.15-0.tgz").build(),
            ArtifactSource::new("vlse-nth-check", "https://registry.npmjs.org/nth-check/-/nth-check-2.1.1.tgz").build(),
            ArtifactSource::new("vlse-picomatch", "https://registry.npmjs.org/picomatch/-/picomatch-2.3.2.tgz").build(),
            ArtifactSource::new("vlse-regenerator-runtime", "https://registry.npmjs.org/regenerator-runtime/-/regenerator-runtime-0.13.11.tgz").build(),
            ArtifactSource::new("vlse-request-light", "https://registry.npmjs.org/request-light/-/request-light-0.7.0.tgz").build(),
            ArtifactSource::new("vlse-semver", "https://registry.npmjs.org/semver/-/semver-7.8.5.tgz").build(),
            ArtifactSource::new("vlse-typescript", "https://registry.npmjs.org/typescript/-/typescript-4.9.5.tgz").build(),
            ArtifactSource::new("vlse-vscode-css-languageservice", "https://registry.npmjs.org/vscode-css-languageservice/-/vscode-css-languageservice-6.3.10.tgz").build(),
            ArtifactSource::new("vlse-vscode-html-languageservice", "https://registry.npmjs.org/vscode-html-languageservice/-/vscode-html-languageservice-5.6.2.tgz").build(),
            ArtifactSource::new("vlse-vscode-json-languageservice", "https://registry.npmjs.org/vscode-json-languageservice/-/vscode-json-languageservice-5.7.2.tgz").build(),
            ArtifactSource::new("vlse-vscode-jsonrpc", "https://registry.npmjs.org/vscode-jsonrpc/-/vscode-jsonrpc-9.0.0-next.1.tgz").build(),
            ArtifactSource::new("vlse-vscode-languageserver", "https://registry.npmjs.org/vscode-languageserver/-/vscode-languageserver-10.0.1.tgz").build(),
            ArtifactSource::new("vlse-vscode-languageserver-protocol", "https://registry.npmjs.org/vscode-languageserver-protocol/-/vscode-languageserver-protocol-3.18.1.tgz").build(),
            ArtifactSource::new("vlse-vscode-languageserver-textdocument", "https://registry.npmjs.org/vscode-languageserver-textdocument/-/vscode-languageserver-textdocument-1.0.13.tgz").build(),
            ArtifactSource::new("vlse-vscode-languageserver-types", "https://registry.npmjs.org/vscode-languageserver-types/-/vscode-languageserver-types-3.18.0.tgz").build(),
            ArtifactSource::new("vlse-vscode-markdown-languageservice", "https://registry.npmjs.org/vscode-markdown-languageservice/-/vscode-markdown-languageservice-0.5.0-alpha.1.tgz").build(),
            ArtifactSource::new("vlse-vscode-nls", "https://registry.npmjs.org/vscode-nls/-/vscode-nls-5.2.0.tgz").build(),
            ArtifactSource::new("vlse-vscode-uri", "https://registry.npmjs.org/vscode-uri/-/vscode-uri-3.1.0.tgz").build(),
        ];

        let env_node = get_env_key(&node.to_string());

        let pkg_dir = format!("$VORPAL_OUTPUT/lib/node_modules/{name}");

        // Copy the package, then assemble a flat node_modules from the vendored dep sources. Node's
        // module resolution walks up from each bin script, so a single flat node_modules at the
        // package root satisfies every server. The closure is conflict-free (no two deps need
        // incompatible majors), so flat resolution is correct.
        let step_script = formatdoc! {"
            mkdir -pv \"{pkg_dir}\" \"$VORPAL_OUTPUT/bin\"

            cp -Rv ./source/{name}/package/* \"{pkg_dir}/.\"

            NM=\"{pkg_dir}/node_modules\"
            mkdir -pv \"$NM\" \"$NM/@vscode\"

            cp -Rv ./source/vlse-vscode-l10n/package \"$NM/@vscode/l10n\"
            cp -Rv ./source/vlse-boolbase/package \"$NM/boolbase\"
            cp -Rv ./source/vlse-core-js/package \"$NM/core-js\"
            cp -Rv ./source/vlse-css-select/package \"$NM/css-select\"
            cp -Rv ./source/vlse-css-what/package \"$NM/css-what\"
            cp -Rv ./source/vlse-dom-serializer/package \"$NM/dom-serializer\"
            cp -Rv ./source/vlse-domelementtype/package \"$NM/domelementtype\"
            cp -Rv ./source/vlse-domhandler/package \"$NM/domhandler\"
            cp -Rv ./source/vlse-domutils/package \"$NM/domutils\"
            cp -Rv ./source/vlse-entities/package \"$NM/entities\"
            cp -Rv ./source/vlse-he/package \"$NM/he\"
            cp -Rv ./source/vlse-jsonc-parser/package \"$NM/jsonc-parser\"
            cp -Rv ./source/vlse-node-html-parser/package \"$NM/node-html-parser\"
            cp -Rv ./source/vlse-nth-check/package \"$NM/nth-check\"
            cp -Rv ./source/vlse-picomatch/package \"$NM/picomatch\"
            cp -Rv ./source/vlse-regenerator-runtime/package \"$NM/regenerator-runtime\"
            cp -Rv ./source/vlse-request-light/package \"$NM/request-light\"
            cp -Rv ./source/vlse-semver/package \"$NM/semver\"
            cp -Rv ./source/vlse-typescript/package \"$NM/typescript\"
            cp -Rv ./source/vlse-vscode-css-languageservice/package \"$NM/vscode-css-languageservice\"
            cp -Rv ./source/vlse-vscode-html-languageservice/package \"$NM/vscode-html-languageservice\"
            cp -Rv ./source/vlse-vscode-json-languageservice/package \"$NM/vscode-json-languageservice\"
            cp -Rv ./source/vlse-vscode-jsonrpc/package \"$NM/vscode-jsonrpc\"
            cp -Rv ./source/vlse-vscode-languageserver/package \"$NM/vscode-languageserver\"
            cp -Rv ./source/vlse-vscode-languageserver-protocol/package \"$NM/vscode-languageserver-protocol\"
            cp -Rv ./source/vlse-vscode-languageserver-textdocument/package \"$NM/vscode-languageserver-textdocument\"
            cp -Rv ./source/vlse-vscode-languageserver-types/package \"$NM/vscode-languageserver-types\"
            cp -Rv ./source/vlse-vscode-markdown-languageservice/package \"$NM/vscode-markdown-languageservice\"
            cp -Rv ./source/vlse-vscode-nls/package \"$NM/vscode-nls\"
            cp -Rv ./source/vlse-vscode-uri/package \"$NM/vscode-uri\"

            for server in css eslint html json markdown; do
                bin_path=\"{pkg_dir}/bin/vscode-$server-language-server\"
                wrapper=\"$VORPAL_OUTPUT/bin/vscode-$server-language-server\"
                cat << EOF > \"$wrapper\"
            #!/bin/sh
            exec {env_node}/bin/node \"$bin_path\" \"\\$@\"
            EOF
                chmod +x \"$wrapper\"
            done",
        };

        // Build-step self-test: prove an assembled wrapper actually starts and resolves the full
        // vendored node_modules by driving a real LSP initialize handshake. The json server is the
        // lightest and exercises the shared vscode-languageserver runtime path that all five share;
        // a MODULE_NOT_FOUND from an incomplete closure would fail the handshake. The driver lives in
        // a raw string (kept out of formatdoc) so its JSON-RPC braces need no escaping.
        let selftest_setup = formatdoc! {"

            echo 'Running LSP initialize self-test against built wrapper...'
            selftest_dir=\"$VORPAL_WORKSPACE/lsp-selftest\"
            mkdir -pv \"$selftest_dir\"
            cat << 'DRIVER_EOF' > \"$selftest_dir/driver.mjs\"
            {SELFTEST_DRIVER_JS}
            DRIVER_EOF
            {env_node}/bin/node \"$selftest_dir/driver.mjs\" \"$VORPAL_OUTPUT/bin/vscode-json-language-server\" \\
                || (echo 'ERROR: vscode-langservers-extracted self-test failed (LSP did not start / resolve deps)' && exit 1)",
            SELFTEST_DRIVER_JS = SELFTEST_DRIVER_JS,
            env_node = env_node,
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
