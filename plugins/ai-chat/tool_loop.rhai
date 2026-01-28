// plugins/ai-chat/tool_loop.rhai

const MAX_TOOL_ITERATIONS = 5;

fn build_tool_call(tc) {
    let func = #{ name: tc.name, arguments: tc.arguments };
    return #{ id: tc.id, type: "function", function: func };
}

fn run_tool_loop(req_id, config, history, tool_defs) {
    let final_response = "";
    let iterations = 0;

    loop {
        iterations += 1;
        if iterations > MAX_TOOL_ITERATIONS {
            final_response = "Error: Too many tool call iterations, stopping.";
            break;
        }

        let response = ai_chat_stream_with_tools(req_id, config, history, tool_defs);

        if type_of(response) == "map" && response.type == "tool_calls" {
            let tool_calls = response.calls ?? [];
            let assistant_msg = #{ role: "assistant", content: (), tool_calls: [] };

            for tc in tool_calls {
                assistant_msg.tool_calls.push(build_tool_call(tc));
            }
            history.push(assistant_msg);

            for tc in tool_calls {
                let result = tools::execute_tool(tc.name, tc.arguments);
                history.push(#{ role: "tool", tool_call_id: tc.id, content: result });
            }
            continue;
        }

        if type_of(response) == "map" && response.type == "text" {
            final_response = response.content ?? "";
        } else if type_of(response) == "string" {
            final_response = response;
        } else {
            final_response = to_json(response);
        }
        break;
    }

    return #{ response: final_response, history: history };
}
