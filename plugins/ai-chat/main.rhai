// AI Chat Plugin
// Implements OpenAI-compatible chat loop with context management
// Phase 3.3: Added sliding window to prevent token overflow
// Phase 3.5: Added Tool Calling support

// Configuration Constants
const MAX_HISTORY_ROUNDS = 10;  // Keep last N conversation rounds (user+assistant pairs)
const MAX_TOOL_ITERATIONS = 5; // Prevent infinite tool call loops

// Sliding Window: Trim history to prevent token overflow
// Invariant: history[0] is always system prompt, preserved.
// Algorithm: Keep system + last MAX_HISTORY_ROUNDS * 2 messages
fn trim_history(history) {
    let max_messages = MAX_HISTORY_ROUNDS * 2 + 1; // +1 for system prompt
    
    while history.len() > max_messages {
        // Remove the oldest non-system message (index 1)
        history.remove(1);
    }
    
    return history;
}

// Built-in tools definition (OpenAI format)
fn get_tools() {
    return [
        #{
            type: "function",
            function: #{
                name: "read_file",
                description: "Read the contents of a file at the given path",
                parameters: #{
                    type: "object",
                    properties: #{
                        path: #{ type: "string", description: "The file path to read" }
                    },
                    required: ["path"]
                }
            }
        },
        #{
            type: "function",
            function: #{
                name: "search_content",
                description: "Search for a pattern in file contents using regex",
                parameters: #{
                    type: "object",
                    properties: #{
                        pattern: #{ type: "string", description: "Regex pattern to search for" },
                        path: #{ type: "string", description: "Directory to search in (default: current)" }
                    },
                    required: ["pattern"]
                }
            }
        }
    ];
}

// Execute a tool call and return the result
fn execute_tool(name, arguments) {
    let args = parse_json(arguments);
    
    if name == "read_file" {
        let path = args.path ?? "";
        if path == "" { return "Error: path is required"; }
        let content = fs_read(path);
        if content == () { return "Error: File not found or cannot be read"; }
        return content;
    }
    
    if name == "search_content" {
        let pattern = args.pattern ?? "";
        if pattern == "" { return "Error: pattern is required"; }
        // For now, return a placeholder - full grep would need host function
        return `Search for "${pattern}" - feature pending full implementation`;
    }
    
    return `Error: Unknown tool "${name}"`;
}

fn chat(req_id, user_message, context) {
    // 1. Ensure history directory exists
    let history_path = ".deve/chat/history.json";
    
    // 2. Load history
    let history_json = fs_read(history_path) ?? "[]";
    let history = parse_json(history_json);
    if type_of(history) != "array" { history = []; }
    
    // 3. Context Injection
    // Build System Prompt with Project Structure
    let tree = get_project_tree();
    
    // Extract current file from context (passed from frontend)
    let current_file = "";
    if type_of(context) == "map" && context.current_file != () {
        current_file = context.current_file;
    }
    
    let system_prompt = `You are Deve-Note AI, an intelligent coding assistant.
    
Current Project Structure:
${tree}

${if current_file != "" { "Currently Editing: " + current_file } else { "" }}

Guidelines:
- Use the project structure to understand file locations.
- When referencing files, use their full paths from the tree.
- If user is editing a file, prioritize context about that file.
- You have access to tools: read_file, search_content. Use them when needed.
- Be concise and helpful.`;

    // Inject or update System Prompt at the beginning of history
    if history.len() == 0 || history[0].role != "system" {
        history.insert(0, #{ role: "system", content: system_prompt });
    } else {
        // Update existing system prompt with latest tree
        history[0].content = system_prompt;
    }
    
    // 4. Append user message
    history.push(#{ role: "user", content: user_message });
    
    // 5. Apply sliding window BEFORE sending to LLM
    history = trim_history(history);
    
    // 6. Prepare config
    let config = #{
        base_url: env("AI_BASE_URL") ?? "https://api.openai.com/v1",
        api_key: env("AI_API_KEY"),
        model: env("AI_MODEL") ?? "gpt-4o",
        max_tokens: 4096,
        headers: #{}
    };
    
    // 6.1 Parse extra headers from environment (JSON format)
    let extra_headers_json = env("AI_EXTRA_HEADERS") ?? "{}";
    let extra_headers = parse_json(extra_headers_json);
    if type_of(extra_headers) == "map" {
        config.headers = extra_headers;
    }
    
    if config.api_key == () {
        return "Error: AI_API_KEY not set in environment.";
    }
    
    // 7. Tool Calling Loop
    let tools = get_tools();
    let final_response = "";
    let iterations = 0;
    
    loop {
        iterations += 1;
        if iterations > MAX_TOOL_ITERATIONS {
            final_response = "Error: Too many tool call iterations, stopping.";
            break;
        }
        
        // Call LLM with tools
        let response = ai_chat_stream(req_id, config, history);
        
        // Check response type
        if type_of(response) == "map" && response.type == "tool_calls" {
            // AI wants to call tools
            let tool_calls = response.calls ?? [];
            
            // Build assistant message with tool_calls
            let assistant_msg = #{
                role: "assistant",
                content: (),
                tool_calls: []
            };
            
            for tc in tool_calls {
                assistant_msg.tool_calls.push(#{
                    id: tc.id,
                    type: "function",
                    function: #{
                        name: tc.name,
                        arguments: tc.arguments
                    }
                });
            }
            history.push(assistant_msg);
            
            // Execute each tool and append results
            for tc in tool_calls {
                let result = execute_tool(tc.name, tc.arguments);
                history.push(#{
                    role: "tool",
                    tool_call_id: tc.id,
                    content: result
                });
            }
            
            // Continue loop to get next response
            continue;
        }
        
        // Normal text response
        if type_of(response) == "map" && response.type == "text" {
            final_response = response.content ?? "";
        } else if type_of(response) == "string" {
            // Backward compatibility: plain string response
            final_response = response;
        } else {
            final_response = to_json(response);
        }
        
        break;
    }
    
    // 8. Append assistant response
    history.push(#{ role: "assistant", content: final_response });
    
    // 9. Trim again after adding response, then save
    history = trim_history(history);
    fs_write(history_path, to_json(history));
    
    return final_response;
}

// Clear chat history
fn clear_history() {
    let history_path = ".deve/chat/history.json";
    fs_write(history_path, "[]");
    return "Chat history cleared.";
}
