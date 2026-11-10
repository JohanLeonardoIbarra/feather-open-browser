// Visual agent overlay and interaction executor for Open Browser.
// Provides a virtual animated cursor (ghost cursor), dynamic Bezier trajectories,
// banking tilt, particle trail, click ripples, element highlighting, and action execution.

const VISUAL_AGENT_SCRIPT_TEMPLATE: &str = include_str!("visual_agent.js");

pub fn build_visual_action_script(action: &serde_json::Value, fast: bool) -> String {
  let action_json = serde_json::to_string(action).unwrap_or_else(|_| "{}".to_string());
  let fast_str = if fast { "true" } else { "false" };

  VISUAL_AGENT_SCRIPT_TEMPLATE
    .replace("__ACTION_JSON__", &action_json)
    .replace("__FAST_BOOL__", fast_str)
}
