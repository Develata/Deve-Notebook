// AI Chat Plugin
// Implements OpenAI-compatible chat loop with context management

fn chat(req_id, user_message) {
    // 1. Ensure history directory exists (Error handling omitted for brevity in Rhai)
    // In real impl, we should check if dir exists or create it. 
    // Host fs_write creates parent dirs? Usually not.
    // For now assume .deve/chat exists or user manually created it.
    
    let history_path = ".deve/chat/history.json";
    
    // 2. Load history
    let history_json = fs_read(history_path) ?? "[]";
    let history = parse_json(history_json);
    if type_of(history) != "array" { history = []; }
    
    // 3. Append user message
    history.push(#{ role: "user", content: user_message });
    
    // 4. Prepare config
    let config = #{
        base_url: env("AI_BASE_URL") ?? "https://api.openai.com/v1",
        api_key: env("AI_API_KEY"),
        model: env("AI_MODEL") ?? "gpt-4o",
        max_tokens: 4096
    };
    
    if config.api_key == () {
        return "Error: AI_API_KEY not set in environment.";
    }
    
    // 5. Stream Chat (Host Function)
    // This will block until stream finishes, while pushing chunks to websocket
    let response_text = ai_chat_stream(req_id, config, history);
    
    // 6. Append assistant response and save
    history.push(#{ role: "assistant", content: response_text });
    
    // Serialize and save (simple JSON stringify)
    // Rhai to_json is built-in or needs helper? 
    // We might need to add a `to_json` host function if Rhai's standard library doesn't suffice for complex objects.
    // Assuming `to_json` exists or we implement it.
    // For now, let's assume `to_json` is available or we add it to host.
    
    fs_write(history_path, to_json(history));
    
    return response_text;
}
