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

const server = spawn(serverBin, ['start'], { stdio: ['pipe', 'pipe', 'pipe'] });

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
pub struct BashLanguageServer<'a> {
    node: Option<&'a str>,
}

impl<'a> BashLanguageServer<'a> {
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

        let name = "bash-language-server";
        let source_version = "5.6.0";

        let source_path =
            format!("https://registry.npmjs.org/{name}/-/{name}-{source_version}.tgz");

        // Runtime dependency closure (production only) of bash-language-server@5.6.0, resolved flat
        // (one version per package). The published tarball ships unbundled out/ that `require`s these
        // at runtime, and vorpal build steps run offline, so each dep is vendored here as a
        // pre-fetched source tarball and copied into a flat node_modules during the build step.
        // Pinned from registry.npmjs.org (the package authors' official publish channel). Source keys
        // are "bls-"-prefixed because vorpal locks sources by key globally, and bare npm names collide
        // with other artifacts' sources.
        let sources = vec![
            ArtifactSource::new(name, &source_path).build(),
            // Direct dependencies
            ArtifactSource::new("bls-editorconfig", "https://registry.npmjs.org/editorconfig/-/editorconfig-2.0.1.tgz").build(),
            ArtifactSource::new("bls-fast-glob", "https://registry.npmjs.org/fast-glob/-/fast-glob-3.3.3.tgz").build(),
            ArtifactSource::new("bls-fuzzy-search", "https://registry.npmjs.org/fuzzy-search/-/fuzzy-search-3.2.1.tgz").build(),
            ArtifactSource::new("bls-node-fetch", "https://registry.npmjs.org/node-fetch/-/node-fetch-2.7.0.tgz").build(),
            ArtifactSource::new("bls-turndown", "https://registry.npmjs.org/turndown/-/turndown-7.2.0.tgz").build(),
            ArtifactSource::new("bls-vscode-languageserver", "https://registry.npmjs.org/vscode-languageserver/-/vscode-languageserver-8.0.2.tgz").build(),
            ArtifactSource::new("bls-vscode-languageserver-textdocument", "https://registry.npmjs.org/vscode-languageserver-textdocument/-/vscode-languageserver-textdocument-1.0.12.tgz").build(),
            ArtifactSource::new("bls-web-tree-sitter", "https://registry.npmjs.org/web-tree-sitter/-/web-tree-sitter-0.24.5.tgz").build(),
            ArtifactSource::new("bls-zod", "https://registry.npmjs.org/zod/-/zod-3.24.2.tgz").build(),
            // editorconfig transitive deps
            ArtifactSource::new("bls-semver", "https://registry.npmjs.org/semver/-/semver-7.8.5.tgz").build(),
            ArtifactSource::new("bls-commander", "https://registry.npmjs.org/commander/-/commander-15.0.0.tgz").build(),
            ArtifactSource::new("bls-minimatch", "https://registry.npmjs.org/minimatch/-/minimatch-10.0.1.tgz").build(),
            ArtifactSource::new("bls-one-ini-wasm", "https://registry.npmjs.org/@one-ini/wasm/-/wasm-0.2.0.tgz").build(),
            ArtifactSource::new("bls-brace-expansion", "https://registry.npmjs.org/brace-expansion/-/brace-expansion-2.0.1.tgz").build(),
            ArtifactSource::new("bls-balanced-match", "https://registry.npmjs.org/balanced-match/-/balanced-match-1.0.2.tgz").build(),
            // fast-glob transitive deps
            ArtifactSource::new("bls-nodelib-fs-stat", "https://registry.npmjs.org/@nodelib/fs.stat/-/fs.stat-2.0.5.tgz").build(),
            ArtifactSource::new("bls-nodelib-fs-walk", "https://registry.npmjs.org/@nodelib/fs.walk/-/fs.walk-1.2.8.tgz").build(),
            ArtifactSource::new("bls-nodelib-fs-scandir", "https://registry.npmjs.org/@nodelib/fs.scandir/-/fs.scandir-2.1.5.tgz").build(),
            ArtifactSource::new("bls-fastq", "https://registry.npmjs.org/fastq/-/fastq-1.20.1.tgz").build(),
            ArtifactSource::new("bls-reusify", "https://registry.npmjs.org/reusify/-/reusify-1.1.0.tgz").build(),
            ArtifactSource::new("bls-run-parallel", "https://registry.npmjs.org/run-parallel/-/run-parallel-1.2.0.tgz").build(),
            ArtifactSource::new("bls-queue-microtask", "https://registry.npmjs.org/queue-microtask/-/queue-microtask-1.2.3.tgz").build(),
            ArtifactSource::new("bls-glob-parent", "https://registry.npmjs.org/glob-parent/-/glob-parent-5.1.2.tgz").build(),
            ArtifactSource::new("bls-is-glob", "https://registry.npmjs.org/is-glob/-/is-glob-4.0.3.tgz").build(),
            ArtifactSource::new("bls-is-extglob", "https://registry.npmjs.org/is-extglob/-/is-extglob-2.1.1.tgz").build(),
            ArtifactSource::new("bls-merge2", "https://registry.npmjs.org/merge2/-/merge2-1.4.1.tgz").build(),
            ArtifactSource::new("bls-micromatch", "https://registry.npmjs.org/micromatch/-/micromatch-4.0.8.tgz").build(),
            ArtifactSource::new("bls-braces", "https://registry.npmjs.org/braces/-/braces-3.0.3.tgz").build(),
            ArtifactSource::new("bls-fill-range", "https://registry.npmjs.org/fill-range/-/fill-range-7.1.1.tgz").build(),
            ArtifactSource::new("bls-to-regex-range", "https://registry.npmjs.org/to-regex-range/-/to-regex-range-5.0.1.tgz").build(),
            ArtifactSource::new("bls-is-number", "https://registry.npmjs.org/is-number/-/is-number-7.0.0.tgz").build(),
            ArtifactSource::new("bls-picomatch", "https://registry.npmjs.org/picomatch/-/picomatch-2.3.1.tgz").build(),
            // node-fetch transitive deps
            ArtifactSource::new("bls-whatwg-url", "https://registry.npmjs.org/whatwg-url/-/whatwg-url-5.0.0.tgz").build(),
            ArtifactSource::new("bls-tr46", "https://registry.npmjs.org/tr46/-/tr46-0.0.3.tgz").build(),
            ArtifactSource::new("bls-webidl-conversions", "https://registry.npmjs.org/webidl-conversions/-/webidl-conversions-3.0.1.tgz").build(),
            // turndown transitive deps
            ArtifactSource::new("bls-mixmark-domino", "https://registry.npmjs.org/@mixmark-io/domino/-/domino-2.2.0.tgz").build(),
            // vscode-languageserver transitive deps
            ArtifactSource::new("bls-vscode-languageserver-protocol", "https://registry.npmjs.org/vscode-languageserver-protocol/-/vscode-languageserver-protocol-3.17.2.tgz").build(),
            ArtifactSource::new("bls-vscode-jsonrpc", "https://registry.npmjs.org/vscode-jsonrpc/-/vscode-jsonrpc-8.0.2.tgz").build(),
            ArtifactSource::new("bls-vscode-languageserver-types", "https://registry.npmjs.org/vscode-languageserver-types/-/vscode-languageserver-types-3.17.2.tgz").build(),
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
            mkdir -pv \"$NM\" \"$NM/@nodelib\" \"$NM/@one-ini\" \"$NM/@mixmark-io\"

            cp -Rv ./source/bls-editorconfig/package \"$NM/editorconfig\"
            cp -Rv ./source/bls-fast-glob/package \"$NM/fast-glob\"
            cp -Rv ./source/bls-fuzzy-search/package \"$NM/fuzzy-search\"
            cp -Rv ./source/bls-node-fetch/package \"$NM/node-fetch\"
            cp -Rv ./source/bls-turndown/package \"$NM/turndown\"
            cp -Rv ./source/bls-vscode-languageserver/package \"$NM/vscode-languageserver\"
            cp -Rv ./source/bls-vscode-languageserver-textdocument/package \"$NM/vscode-languageserver-textdocument\"
            cp -Rv ./source/bls-web-tree-sitter/package \"$NM/web-tree-sitter\"
            cp -Rv ./source/bls-zod/package \"$NM/zod\"
            cp -Rv ./source/bls-semver/package \"$NM/semver\"
            cp -Rv ./source/bls-commander/package \"$NM/commander\"
            cp -Rv ./source/bls-minimatch/package \"$NM/minimatch\"
            cp -Rv ./source/bls-one-ini-wasm/package \"$NM/@one-ini/wasm\"
            cp -Rv ./source/bls-brace-expansion/package \"$NM/brace-expansion\"
            cp -Rv ./source/bls-balanced-match/package \"$NM/balanced-match\"
            cp -Rv ./source/bls-nodelib-fs-stat/package \"$NM/@nodelib/fs.stat\"
            cp -Rv ./source/bls-nodelib-fs-walk/package \"$NM/@nodelib/fs.walk\"
            cp -Rv ./source/bls-nodelib-fs-scandir/package \"$NM/@nodelib/fs.scandir\"
            cp -Rv ./source/bls-fastq/package \"$NM/fastq\"
            cp -Rv ./source/bls-reusify/package \"$NM/reusify\"
            cp -Rv ./source/bls-run-parallel/package \"$NM/run-parallel\"
            cp -Rv ./source/bls-queue-microtask/package \"$NM/queue-microtask\"
            cp -Rv ./source/bls-glob-parent/package \"$NM/glob-parent\"
            cp -Rv ./source/bls-is-glob/package \"$NM/is-glob\"
            cp -Rv ./source/bls-is-extglob/package \"$NM/is-extglob\"
            cp -Rv ./source/bls-merge2/package \"$NM/merge2\"
            cp -Rv ./source/bls-micromatch/package \"$NM/micromatch\"
            cp -Rv ./source/bls-braces/package \"$NM/braces\"
            cp -Rv ./source/bls-fill-range/package \"$NM/fill-range\"
            cp -Rv ./source/bls-to-regex-range/package \"$NM/to-regex-range\"
            cp -Rv ./source/bls-is-number/package \"$NM/is-number\"
            cp -Rv ./source/bls-picomatch/package \"$NM/picomatch\"
            cp -Rv ./source/bls-whatwg-url/package \"$NM/whatwg-url\"
            cp -Rv ./source/bls-tr46/package \"$NM/tr46\"
            cp -Rv ./source/bls-webidl-conversions/package \"$NM/webidl-conversions\"
            cp -Rv ./source/bls-mixmark-domino/package \"$NM/@mixmark-io/domino\"
            cp -Rv ./source/bls-vscode-languageserver-protocol/package \"$NM/vscode-languageserver-protocol\"
            cp -Rv ./source/bls-vscode-jsonrpc/package \"$NM/vscode-jsonrpc\"
            cp -Rv ./source/bls-vscode-languageserver-types/package \"$NM/vscode-languageserver-types\"

            cat << EOF > \"$VORPAL_OUTPUT/bin/{name}\"
            #!/bin/sh
            exec {env_node}/bin/node \"{pkg_dir}/out/cli.js\" \"\\$@\"
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
                || (echo 'ERROR: bash-language-server self-test failed (LSP did not start / resolve deps)' && exit 1)",
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
