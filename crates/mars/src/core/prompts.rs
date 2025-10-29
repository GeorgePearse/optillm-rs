/// Prompt templates for MARS stages.
///
/// These prompts are derived from the optillm MARS implementation
/// and guide agents through reasoning, verification, and improvement.

/// System prompt for agents in MARS (with thinking tags)
pub const MARS_SYSTEM_PROMPT_WITH_THINKING: &str = r#"You are a helpful assistant tasked with solving complex problems.
Use careful reasoning and break down problems into steps.
Before providing your final answer, wrap your reasoning in <think></think> tags.

Format your response as:
<think>
[Your step-by-step reasoning here]
</think>

[Final answer here]"#;

/// System prompt for agents in MARS (without thinking tags)
pub const MARS_SYSTEM_PROMPT: &str = r#"You are a helpful assistant tasked with solving complex problems.
Think through each step carefully and provide a well-reasoned answer.
Your goal is to arrive at the correct solution through systematic analysis."#;

/// Initial reasoning prompt for agents
pub const MARS_REASONING_PROMPT: &str = r#"Please solve the following problem step by step.
Show all your work and reasoning. Be thorough and systematic.
Consider edge cases and verify your logic at each step."#;

/// System prompt for the verification agent
pub const VERIFICATION_SYSTEM_PROMPT: &str = r#"You are an expert verifier tasked with evaluating solutions.
Assess the provided solution for:
1. Mathematical correctness - Is the answer actually correct?
2. Completeness - Does the solution address all aspects of the problem?
3. Rigor - Is the reasoning sound and well-justified?
4. Clarity - Is the solution easy to follow?

Provide a verification result: CORRECT or INCORRECT
Also provide a confidence score from 0.0 to 1.0.

Format your response as:
RESULT: CORRECT|INCORRECT
SCORE: [0.0-1.0]
FEEDBACK: [Your detailed feedback]"#;

/// Prompt for improving unverified solutions
pub const IMPROVEMENT_PROMPT: &str = r#"The previous solution needs improvement.
Please revise it to address the feedback provided.
Be particularly careful to fix any errors in reasoning.
Provide your improved solution with clear step-by-step reasoning."#;

/// Prompt for aggregating multiple solutions
pub const AGGREGATION_PROMPT: &str = r#"You are given multiple solutions to the same problem.
Your task is to synthesize the best elements from each solution.

Solutions:
{solutions}

Please create an improved solution that:
1. Takes the best elements from each approach
2. Corrects any errors found in individual solutions
3. Provides clear, step-by-step reasoning
4. Arrives at the most likely correct answer

Synthesized solution:"#;

/// Prompt for extracting strategies from successful solutions
pub const STRATEGY_EXTRACTION_PROMPT: &str = r#"Analyze the following successful solution and identify key strategies and techniques used.

Solution:
{solution}

Please identify and list 3-5 key strategies or techniques that contributed to solving this problem well.
Format as a numbered list with brief explanations."#;

/// Prompt for cross-agent strategy sharing
pub const STRATEGY_SHARING_PROMPT: &str = r#"You have access to strategies that other agents have successfully used:

{strategies}

Use these insights to improve your reasoning approach. Which of these strategies might be helpful?
How can you incorporate them into solving the current problem?"#;

/// Prompt for final synthesis when no consensus is reached
pub const SYNTHESIS_PROMPT: &str = r#"Multiple reasoning approaches have been tried for this problem:

{solutions}

Please synthesize a final answer that:
1. Considers all approaches
2. Identifies the most likely correct answer
3. Explains why this answer is most reliable

Final answer with explanation:"#;

/// Prompt template for specialized mathematical reasoning
pub const MATH_REASONING_PROMPT: &str = r#"Solve this mathematical problem step by step.
Show all calculations and intermediate results.
Verify your answer by substituting back if possible.

Problem: {problem}"#;

/// Prompt template for coding problems
pub const CODE_REASONING_PROMPT: &str = r#"Write code to solve this problem.
Include clear comments explaining your logic.
Consider edge cases and test your solution mentally.

Problem: {problem}"#;

/// Prompt template for logical reasoning
pub const LOGIC_REASONING_PROMPT: &str = r#"Analyze this logical problem carefully.
Break it down into components.
Use systematic reasoning to reach conclusions.

Problem: {problem}"#;

/// Prompt for confidence assessment
pub const CONFIDENCE_ASSESSMENT_PROMPT: &str = r#"Rate your confidence in this answer on a scale of 0.0 (no confidence) to 1.0 (absolute confidence).
Explain what would make you more confident in this answer.
Are there any uncertainties or edge cases you're not sure about?"#;

/// Instruction for thinking tag extraction
pub const EXTRACT_ANSWER_INSTRUCTION: &str = r#"Extract the final answer from the reasoning.
If the response is wrapped in <think></think> tags, extract the content after those tags.
The answer should be concise and directly answerable.
If no clear answer is provided, indicate that the answer could not be extracted."#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompts_not_empty() {
        assert!(!MARS_SYSTEM_PROMPT.is_empty());
        assert!(!MARS_REASONING_PROMPT.is_empty());
        assert!(!VERIFICATION_SYSTEM_PROMPT.is_empty());
        assert!(!IMPROVEMENT_PROMPT.is_empty());
    }

    #[test]
    fn test_thinking_tags_in_prompt() {
        assert!(MARS_SYSTEM_PROMPT_WITH_THINKING.contains("think"));
    }
}
