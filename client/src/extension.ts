/* --------------------------------------------------------------------------------------------
 * Copyright (c) Microsoft Corporation. All rights reserved.
 * Licensed under the MIT License. See License.txt in the project root for license information.
 * ------------------------------------------------------------------------------------------ */
import * as path from "path";
import { ExtensionContext, workspace, window} from "vscode";
import {
  Executable,
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";
let client: LanguageClient;
export function activate(context: ExtensionContext) {

  window.showInformationMessage("SimplicityHL LSP activated!");
 
  const command = "simplicityhl-lsp";
  const run: Executable = {
    command,
    options: {
      env: {
        ...process.env,
        // eslint-disable-next-line @typescript-eslint/naming-convention
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
