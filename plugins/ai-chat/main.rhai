// AI Chat Plugin
// Implements OpenAI-compatible chat loop with context management

fn chat(req_id, user_message) {
    // 1. Ensure history directory exists
    let history_path = ".deve/chat/history.json";
    
    // 2. Load history
    let history_json = fs_read(history_path) ?? "[]";
    let history = parse_json(history_json);
    if type_of(history) != "array" { history = []; }
    
    // 3. Context Injection (NEW)
    // Build System Prompt with Project Structure
    let tree = get_project_tree();
    let system_prompt = `You are Deve-Note AI, an intelligent coding assistant.
    
Current Project Structure:
${tree}

Guidelines:
- Use the project structure to understand file locations.
- When referencing files, use their full paths from the tree.
- Be concise and helpful.`;

    // Inject or update System Prompt at the beginning of history
    // If history is empty or first message isn't system, prepend it.
    if history.len() == 0 || history[0].role != "system" {
        history.insert(0, #{ role: "system", content: system_prompt });
    } else {
        // Update existing system prompt with latest tree
        history[0].content = system_prompt;
    }
    
    // 4. Append user message
    history.push(#{ role: "user", content: user_message });
    
    // 5. Prepare config
    let config = #{
        base_url: env("AI_BASE_URL") ?? "https://api.openai.com/v1",
        api_key: env("AI_API_KEY"),
        model: env("AI_MODEL") ?? "gpt-4o",
        max_tokens: 4096
    };
    
    if config.api_key == () {
        return "Error: AI_API_KEY not set in environment.";
    }
    
    // 6. Stream Chat (Host Function)
    let response_text = ai_chat_stream(req_id, config, history);
    
    // 7. Append assistant response and save
    history.push(#{ role: "assistant", content: response_text });
    
    // Save history (Limit history length to save tokens? TODO)
    fs_write(history_path, to_json(history));
    
    return response_text;
}
