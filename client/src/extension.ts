/* --------------------------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 * ------------------------------------------------------------------------------------------ */
import * as path from "path";
import { ExtensionContext, window, workspace } from "vscode";
import {
  Executable,
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";
import * as cp from "child_process";
let client: LanguageClient;
export function activate(context: ExtensionContext) {
  const command = "simplicityhl-lsp";
  try {
    cp.execSync(
      process.platform === "win32" ? `where ${command}` : `which ${command}`,
    );
  } catch {
    window.showErrorMessage(
      `LSP server "${command}" was not found in your PATH. Please install it or add it to PATH.`,
    );
    return;
  }
  window.showInformationMessage("SimplicityHL LSP activated!");
  const run: Executable = {
    command,
    options: {
      env: {
        ...process.env,
        RUST_LOG: "debug",
      },
    },
  };
  // If the extension is launched in debug mode then the debug server options are used
  // Otherwise the run options are used
  const serverOptions: ServerOptions = {
    run,
    debug: run,
  };
  // Options to control the language client
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "simplicityhl" }],
  };
  // Create the language client and start the client.
  client = new LanguageClient(
    "simplicityhl-lsp",
    "SimplicityHL LSP",
    serverOptions,
    clientOptions,
  );
  // Start the client. This will also launch the server
  client.start();
}
export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
