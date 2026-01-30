/**
 * Deve-Note Web Initialization
 * 
 * Registers default code block actions.
 */
if (typeof window !== 'undefined') {
    window.deve_code_actions = window.deve_code_actions || [];

    // Action: Run Code (Console)
    window.deve_code_actions.push({
        id: 'run-code',
        label: 'Run Code',
        run: ({ code, language }) => {
            console.group(`Run Code (${language || 'unknown'})`);
            console.log(code);
            console.groupEnd();
            // TODO: Connect to backend or WASM runtime
        }
    });

    // Action: Send to AI
    window.deve_code_actions.push({
        id: 'send-ai',
        label: 'Send to AI',
        run: ({ code }) => {
            console.log('[AI] Sending code to AI context...');
            console.log(code);
            // TODO: Trigger AI Sidebar with code context
        }
    });
}
