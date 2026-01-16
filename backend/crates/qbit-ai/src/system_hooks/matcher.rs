//! Hook matchers for system hooks.
//!
//! Matchers determine when a hook should fire based on context.

use qbit_core::{ToolCategory, ToolName};
use regex::Regex;

use super::context::{MessageHookContext, MessageType, PostToolContext, PreToolContext};

/// Matcher for message-based hooks.
///
/// Determines when a hook should fire based on message content and type.
pub enum MessageMatcher {
    /// Match if the message contains a keyword (case-insensitive).
    Keyword(String),

    /// Match if the message matches a regex pattern.
    Regex(Regex),

    /// Match if the message is of a specific type.
    MessageType(MessageType),

    /// Custom predicate function.
    Custom(fn(&MessageHookContext) -> bool),

    /// All matchers must match (AND).
    All(Vec<MessageMatcher>),

    /// Any matcher must match (OR).
    Any(Vec<MessageMatcher>),
}

impl MessageMatcher {
    /// Create a keyword matcher (case-insensitive).
    pub fn keyword(keyword: impl Into<String>) -> Self {
        Self::Keyword(keyword.into().to_lowercase())
    }

    /// Create a regex matcher.
    pub fn regex(pattern: &str) -> Result<Self, regex::Error> {
        Ok(Self::Regex(Regex::new(pattern)?))
    }

    /// Create a message type matcher.
    pub fn message_type(msg_type: MessageType) -> Self {
        Self::MessageType(msg_type)
    }

    /// Create a custom matcher.
    pub fn custom(predicate: fn(&MessageHookContext) -> bool) -> Self {
        Self::Custom(predicate)
    }

    /// Combine matchers with AND logic.
    pub fn all(matchers: Vec<MessageMatcher>) -> Self {
        Self::All(matchers)
    }

    /// Combine matchers with OR logic.
    pub fn any(matchers: Vec<MessageMatcher>) -> Self {
        Self::Any(matchers)
    }

    /// Check if this matcher matches the given context.
    pub fn matches(&self, ctx: &MessageHookContext) -> bool {
        match self {
            Self::Keyword(kw) => ctx.content.to_lowercase().contains(kw),
            Self::Regex(re) => re.is_match(ctx.content),
            Self::MessageType(mt) => ctx.message_type == *mt,
            Self::Custom(f) => f(ctx),
            Self::All(matchers) => matchers.iter().all(|m| m.matches(ctx)),
            Self::Any(matchers) => matchers.iter().any(|m| m.matches(ctx)),
        }
    }
}

impl std::fmt::Debug for MessageMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Keyword(kw) => write!(f, "Keyword({:?})", kw),
            Self::Regex(re) => write!(f, "Regex({:?})", re.as_str()),
            Self::MessageType(mt) => write!(f, "MessageType({:?})", mt),
            Self::Custom(_) => write!(f, "Custom(<fn>)"),
            Self::All(matchers) => write!(f, "All({:?})", matchers),
            Self::Any(matchers) => write!(f, "Any({:?})", matchers),
        }
    }
}

/// Matcher for tool-based hooks.
///
/// Determines when a hook should fire based on tool execution context.
pub enum ToolMatcher {
    /// Match a specific tool by name.
    Tool(ToolName),

    /// Match any tool in a category.
    Category(ToolCategory),

    /// Match any of the specified tools.
    Tools(Vec<ToolName>),

    /// Match sub-agent tools.
    SubAgent,

    /// Match a specific sub-agent by ID.
    SubAgentId(String),

    /// Custom predicate for pre-tool context.
    CustomPre(fn(&PreToolContext) -> bool),

    /// Custom predicate for post-tool context.
    CustomPost(fn(&PostToolContext) -> bool),

    /// All matchers must match (AND) - for pre-tool context.
    AllPre(Vec<ToolMatcher>),

    /// Any matcher must match (OR) - for pre-tool context.
    AnyPre(Vec<ToolMatcher>),

    /// All matchers must match (AND) - for post-tool context.
    AllPost(Vec<ToolMatcher>),

    /// Any matcher must match (OR) - for post-tool context.
    AnyPost(Vec<ToolMatcher>),
}

impl ToolMatcher {
    /// Create a tool matcher for a specific tool.
    pub fn tool(tool: ToolName) -> Self {
        Self::Tool(tool)
    }

    /// Create a category matcher.
    pub fn category(category: ToolCategory) -> Self {
        Self::Category(category)
    }

    /// Create a matcher for multiple tools.
    pub fn tools(tools: Vec<ToolName>) -> Self {
        Self::Tools(tools)
    }

    /// Create a matcher for any sub-agent.
    pub fn sub_agent() -> Self {
        Self::SubAgent
    }

    /// Create a matcher for a specific sub-agent.
    pub fn sub_agent_id(id: impl Into<String>) -> Self {
        Self::SubAgentId(id.into())
    }

    /// Create a custom pre-tool matcher.
    pub fn custom_pre(predicate: fn(&PreToolContext) -> bool) -> Self {
        Self::CustomPre(predicate)
    }

    /// Create a custom post-tool matcher.
    pub fn custom_post(predicate: fn(&PostToolContext) -> bool) -> Self {
        Self::CustomPost(predicate)
    }

    /// Check if this matcher matches the given pre-tool context.
    pub fn matches_pre(&self, ctx: &PreToolContext) -> bool {
        match self {
            Self::Tool(t) => ctx.tool == Some(*t),
            Self::Category(c) => ctx.tool.map(|t| t.category() == *c).unwrap_or(false),
            Self::Tools(tools) => ctx.tool.map(|t| tools.contains(&t)).unwrap_or(false),
            Self::SubAgent => ctx.is_sub_agent(),
            Self::SubAgentId(id) => ctx.sub_agent_id() == Some(id.as_str()),
            Self::CustomPre(f) => f(ctx),
            Self::CustomPost(_) => false, // Post-tool matcher doesn't apply to pre-tool context
            Self::AllPre(matchers) => matchers.iter().all(|m| m.matches_pre(ctx)),
            Self::AnyPre(matchers) => matchers.iter().any(|m| m.matches_pre(ctx)),
            Self::AllPost(_) | Self::AnyPost(_) => false,
        }
    }

    /// Check if this matcher matches the given post-tool context.
    pub fn matches_post(&self, ctx: &PostToolContext) -> bool {
        match self {
            Self::Tool(t) => ctx.tool == Some(*t),
            Self::Category(c) => ctx.tool.map(|t| t.category() == *c).unwrap_or(false),
            Self::Tools(tools) => ctx.tool.map(|t| tools.contains(&t)).unwrap_or(false),
            Self::SubAgent => ctx.is_sub_agent(),
            Self::SubAgentId(id) => ctx.sub_agent_id() == Some(id.as_str()),
            Self::CustomPre(_) => false, // Pre-tool matcher doesn't apply to post-tool context
            Self::CustomPost(f) => f(ctx),
            Self::AllPre(_) | Self::AnyPre(_) => false,
            Self::AllPost(matchers) => matchers.iter().all(|m| m.matches_post(ctx)),
            Self::AnyPost(matchers) => matchers.iter().any(|m| m.matches_post(ctx)),
        }
    }
}

impl std::fmt::Debug for ToolMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tool(t) => write!(f, "Tool({:?})", t),
            Self::Category(c) => write!(f, "Category({:?})", c),
            Self::Tools(tools) => write!(f, "Tools({:?})", tools),
            Self::SubAgent => write!(f, "SubAgent"),
            Self::SubAgentId(id) => write!(f, "SubAgentId({:?})", id),
            Self::CustomPre(_) => write!(f, "CustomPre(<fn>)"),
            Self::CustomPost(_) => write!(f, "CustomPost(<fn>)"),
            Self::AllPre(matchers) => write!(f, "AllPre({:?})", matchers),
            Self::AnyPre(matchers) => write!(f, "AnyPre({:?})", matchers),
            Self::AllPost(matchers) => write!(f, "AllPost({:?})", matchers),
            Self::AnyPost(matchers) => write!(f, "AnyPost({:?})", matchers),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_message_matcher_keyword() {
        let matcher = MessageMatcher::keyword("error");
        let ctx = MessageHookContext::user_input("There was an ERROR in the code", "s1");
        assert!(matcher.matches(&ctx));

        let ctx = MessageHookContext::user_input("Everything is fine", "s1");
        assert!(!matcher.matches(&ctx));
    }

    #[test]
    fn test_message_matcher_regex() {
        let matcher = MessageMatcher::regex(r"\berror\b").unwrap();
        let ctx = MessageHookContext::user_input("An error occurred", "s1");
        assert!(matcher.matches(&ctx));

        let ctx = MessageHookContext::user_input("No errors here", "s1");
        assert!(!matcher.matches(&ctx)); // "errors" doesn't match \berror\b
    }

    #[test]
    fn test_message_matcher_type() {
        let matcher = MessageMatcher::message_type(MessageType::UserInput);

        let ctx = MessageHookContext::user_input("hello", "s1");
        assert!(matcher.matches(&ctx));

        let ctx = MessageHookContext::agent_response("hello", "s1");
        assert!(!matcher.matches(&ctx));
    }

    #[test]
    fn test_message_matcher_all() {
        let matcher = MessageMatcher::all(vec![
            MessageMatcher::keyword("urgent"),
            MessageMatcher::message_type(MessageType::UserInput),
        ]);

        let ctx = MessageHookContext::user_input("This is URGENT", "s1");
        assert!(matcher.matches(&ctx));

        let ctx = MessageHookContext::agent_response("This is URGENT", "s1");
        assert!(!matcher.matches(&ctx)); // wrong type

        let ctx = MessageHookContext::user_input("Not important", "s1");
        assert!(!matcher.matches(&ctx)); // missing keyword
    }

    #[test]
    fn test_message_matcher_any() {
        let matcher = MessageMatcher::any(vec![
            MessageMatcher::keyword("error"),
            MessageMatcher::keyword("warning"),
        ]);

        let ctx = MessageHookContext::user_input("There's an error", "s1");
        assert!(matcher.matches(&ctx));

        let ctx = MessageHookContext::user_input("Just a warning", "s1");
        assert!(matcher.matches(&ctx));

        let ctx = MessageHookContext::user_input("All good", "s1");
        assert!(!matcher.matches(&ctx));
    }

    #[test]
    fn test_tool_matcher_tool() {
        let matcher = ToolMatcher::tool(ToolName::ReadFile);
        let args = json!({});

        let ctx = PreToolContext::new("read_file", &args, "s1");
        assert!(matcher.matches_pre(&ctx));

        let ctx = PreToolContext::new("write_file", &args, "s1");
        assert!(!matcher.matches_pre(&ctx));
    }

    #[test]
    fn test_tool_matcher_category() {
        let matcher = ToolMatcher::category(ToolCategory::FileOps);
        let args = json!({});

        let ctx = PreToolContext::new("read_file", &args, "s1");
        assert!(matcher.matches_pre(&ctx));

        let ctx = PreToolContext::new("write_file", &args, "s1");
        assert!(matcher.matches_pre(&ctx));

        let ctx = PreToolContext::new("run_pty_cmd", &args, "s1");
        assert!(!matcher.matches_pre(&ctx));
    }

    #[test]
    fn test_tool_matcher_sub_agent() {
        let matcher = ToolMatcher::sub_agent();
        let args = json!({});

        let ctx = PreToolContext::new("sub_agent_coder", &args, "s1");
        assert!(matcher.matches_pre(&ctx));

        let ctx = PreToolContext::new("read_file", &args, "s1");
        assert!(!matcher.matches_pre(&ctx));
    }

    #[test]
    fn test_tool_matcher_post() {
        let matcher = ToolMatcher::tool(ToolName::UpdatePlan);
        let args = json!({});
        let result = json!({"success": true});

        let ctx = PostToolContext::new("update_plan", &args, &result, true, 100, "s1");
        assert!(matcher.matches_post(&ctx));
    }
}
