import * as vscode from 'vscode';
import * as cp from 'child_process';
import * as path from 'path';

export function activate(context: vscode.ExtensionContext) {
    console.log('Abyss extension active');

    // Command: Generate Context
    let disposable = vscode.commands.registerCommand('abyss.generateContext', async () => {
        await runAbyss(false);
    });

    // Command: Generate to Clipboard
    let disposableClipboard = vscode.commands.registerCommand('abyss.generateClipboard', async () => {
        await runAbyss(true);
    });

    context.subscriptions.push(disposable);
    context.subscriptions.push(disposableClipboard);
}

async function runAbyss(clipboard: boolean) {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders) {
        vscode.window.showErrorMessage('No workspace folder open.');
        return;
    }

    const rootPath = workspaceFolders[0].uri.fsPath;
    const config = vscode.workspace.getConfiguration('abyss');
    const format = config.get<string>('format', 'markdown');
    const maxTokens = config.get<number>('maxTokens', 128000);

    const ext = format === 'markdown' ? 'md' : format;
    const outputFile = path.join(rootPath, `abyss-context.${ext}`);

    // Construct args
    const args = ['.', '-o', outputFile, '--format', format, '--max-tokens', maxTokens.toString()];
    if (clipboard) {
        args.push('--clipboard');
    }

    vscode.window.withProgress({
        location: vscode.ProgressLocation.Notification,
        title: "Abyss: Generating Context...",
        cancellable: false
    }, async (progress) => {
        return new Promise<void>((resolve, reject) => {
            const child = cp.spawn('abyss', args, { cwd: rootPath });

            child.on('error', (err) => {
                vscode.window.showErrorMessage(`Failed to start Abyss: ${err.message}. Is 'abyss' installed? (npm install -g abyss-cli)`);
                resolve();
            });

            child.on('close', (code) => {
                if (code === 0) {
                    if (clipboard) {
                        vscode.window.showInformationMessage('Abyss Context copied to clipboard!');
                    } else {
                        vscode.window.showInformationMessage(`Context generated: ${outputFile}`);
                        vscode.workspace.openTextDocument(outputFile).then(doc => {
                            vscode.window.showTextDocument(doc);
                        });
                    }
                } else {
                    vscode.window.showErrorMessage(`Abyss failed with exit code ${code}`);
                }
                resolve();
            });
        });
    });
}

export function deactivate() {}
