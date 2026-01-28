// plugins/ai-chat/state.rhai

const STATE_PATH = ".deve/chat/state.json";

fn load_state() {
    let raw = fs_read(STATE_PATH) ?? "{}";
    let state = parse_json(raw);
    if type_of(state) != "map" { state = #{}; }
    if type_of(state.active_skills) != "array" { state.active_skills = []; }
    return state;
}

fn save_state(state) {
    fs_write(STATE_PATH, to_json(state));
}

fn mode_plan() { return "plan"; }
fn mode_build() { return "build"; }

fn get_mode() {
    let state = load_state();
    let mode = state.mode ?? mode_build();
    if mode != mode_plan() && mode != mode_build() { mode = mode_build(); }
    return mode;
}

fn set_mode(mode) {
    let state = load_state();
    state.mode = mode;
    save_state(state);
}

fn get_active_skills() {
    let state = load_state();
    return state.active_skills ?? [];
}

fn add_active_skill(name) {
    let state = load_state();
    let skills = state.active_skills ?? [];
    if !skills.contains(name) {
        skills.push(name);
        state.active_skills = skills;
        save_state(state);
    }
}
