/// MOA (Mixture of Agents) strategy for solution aggregation.
///
/// This module implements the MOA algorithm from optillm:
/// https://github.com/raidium/optillm
///
/// The algorithm works in three phases:
/// 1. Generate 3 diverse completions with high temperature
/// 2. Critique each completion, analyzing strengths and weaknesses
/// 3. Synthesize final answer using critiques and candidates
///
/// Based on references/optillm/optillm/moa.py

use crate::types::Solution;
use crate::Result;
use futures::StreamExt;

/// MOA aggregator implementing the Mixture of Agents algorithm
pub struct MoaAggregator;

/// Metadata about MOA aggregation
#[derive(Debug, Clone)]
pub struct MoaMetadata {
    /// Total completion tokens used across all phases
    pub total_tokens: usize,
    /// Tokens used in phase 1 (initial completions)
    pub phase1_tokens: usize,
    /// Tokens used in phase 2 (critique)
    pub phase2_tokens: usize,
    /// Tokens used in phase 3 (synthesis)
    pub phase3_tokens: usize,
    /// Number of completions generated
    pub num_completions: usize,
    /// Whether fallback was used
    pub fallback_used: bool,
}

impl MoaAggregator {
    /// Generate N initial completions (Phase 1)
    ///
    /// Generates N completions sequentially using the provided LLM provider.
    /// If fewer than N completions succeed, pads with the first completion.
    async fn generate_initial_completions(
        query: &str,
        system_prompt: &str,
        num_completions: usize,
        client: &dyn crate::ModelClient,
        fallback_enabled: bool,
    ) -> Result<(Vec<String>, usize, bool)> {
        let mut completions = Vec::new();
        let mut total_tokens = 0;
        let mut fallback_used = false;

        // Generate completions sequentially
        for i in 0..num_completions {
            let mut prompt = crate::Prompt::default();
            prompt.input = vec![crate::ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![crate::ContentItem::InputText {
                    text: query.to_string(),
                }],
            }];
            prompt.base_instructions_override = Some(system_prompt.to_string());

            let mut stream = client.stream(&prompt);
            let mut response = String::new();
            while let Some(event) = stream.next().await {
                match event {
                    Ok(crate::ResponseEvent::OutputTextDelta { delta, .. }) => {
                        response.push_str(&delta);
                        total_tokens += 1; // Rough estimate
                    }
                    Ok(crate::ResponseEvent::Completed { .. }) => break,
                    Err(e) => {
                        if !fallback_enabled {
                            return Err(crate::MarsError::AggregationError(format!(
                                "Failed to generate completion {}: {}",
                                i + 1,
                                e
                            )));
                        }
                        // Continue trying other completions even if one fails
                        fallback_used = true;
                        break;
                    }
                    _ => {}
                }
            }
            if !response.is_empty() {
                completions.push(response);
            }
        }

        // If we have no completions, return error
        if completions.is_empty() {
            return Err(crate::MarsError::AggregationError(
                "Failed to generate any completions in MOA phase 1".to_string(),
            ));
        }

        // Pad with first completion if needed
        while completions.len() < num_completions && !completions.is_empty() {
            completions.push(completions[0].clone());
        }

        Ok((completions, total_tokens, fallback_used))
    }

    /// Generate critique of all completions (Phase 2)
    async fn generate_critique(
        query: &str,
        completions: &[String],
        system_prompt: &str,
        client: &dyn crate::ModelClient,
    ) -> Result<(String, usize)> {
        if completions.len() < 3 {
            return Err(crate::MarsError::AggregationError(
                "MOA requires at least 3 completions for critique".to_string(),
            ));
        }

        let critique_prompt = format!(
            "Original query: {}\n\n\
             I will present you with three candidate responses to the original query. \
             Please analyze and critique each response, discussing their strengths and weaknesses. \
             Provide your analysis for each candidate separately.\n\n\
             Candidate 1:\n{}\n\n\
             Candidate 2:\n{}\n\n\
             Candidate 3:\n{}\n\n\
             Please provide your critique for each candidate:",
            query, completions[0], completions[1], completions[2]
        );

        let mut prompt = crate::Prompt::default();
        prompt.input = vec![crate::ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![crate::ContentItem::InputText {
                text: critique_prompt.clone(),
            }],
        }];
        prompt.base_instructions_override = Some(system_prompt.to_string());

        let mut critique = String::new();
        let mut stream = client.stream(&prompt);
        let mut token_count = 0;
        while let Some(event) = stream.next().await {
            match event {
                Ok(crate::ResponseEvent::OutputTextDelta { delta, .. }) => {
                    critique.push_str(&delta);
                    token_count += 1;
                }
                Ok(crate::ResponseEvent::Completed { .. }) => break,
                Err(e) => {
                    return Err(crate::MarsError::AggregationError(format!(
                        "Failed to generate critique in MOA phase 2: {}",
                        e
                    )))
                }
                _ => {}
            }
        }

        if critique.is_empty() {
            return Err(crate::MarsError::AggregationError(
                "Failed to generate critique in MOA phase 2".to_string(),
            ));
        }

        Ok((critique, token_count))
    }

    /// Generate final synthesis (Phase 3)
    async fn generate_final_synthesis(
        query: &str,
        completions: &[String],
        critique: &str,
        system_prompt: &str,
        client: &dyn crate::ModelClient,
    ) -> Result<(String, usize)> {
        if completions.len() < 3 {
            return Err(crate::MarsError::AggregationError(
                "MOA requires at least 3 completions for synthesis".to_string(),
            ));
        }

        let synthesis_prompt = format!(
            "Original query: {}\n\n\
             Based on the following candidate responses and their critiques, \
             generate a final response to the original query.\n\n\
             Candidate 1:\n{}\n\n\
             Candidate 2:\n{}\n\n\
             Candidate 3:\n{}\n\n\
             Critiques of all candidates:\n{}\n\n\
             Please provide a final, optimized response to the original query:",
            query, completions[0], completions[1], completions[2], critique
        );

        let mut prompt = crate::Prompt::default();
        prompt.input = vec![crate::ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![crate::ContentItem::InputText {
                text: synthesis_prompt.clone(),
            }],
        }];
        prompt.base_instructions_override = Some(system_prompt.to_string());

        let mut synthesis = String::new();
        let mut stream = client.stream(&prompt);
        let mut token_count = 0;
        while let Some(event) = stream.next().await {
            match event {
                Ok(crate::ResponseEvent::OutputTextDelta { delta, .. }) => {
                    synthesis.push_str(&delta);
                    token_count += 1;
                }
                Ok(crate::ResponseEvent::Completed { .. }) => break,
                Err(e) => {
                    return Err(crate::MarsError::AggregationError(format!(
                        "Failed to generate final synthesis in MOA phase 3: {}",
                        e
                    )))
                }
                _ => {}
            }
        }

        if synthesis.is_empty() {
            return Err(crate::MarsError::AggregationError(
                "Failed to generate final synthesis in MOA phase 3".to_string(),
            ));
        }

        Ok((synthesis, token_count))
    }

    /// Run the full MOA aggregation pipeline
    ///
    /// Supports the optillm_core ModelClient interface for flexible provider support.
    pub async fn run_moa(
        query: &str,
        system_prompt: &str,
        num_completions: usize,
        fallback_enabled: bool,
        client: &dyn crate::ModelClient,
    ) -> Result<(Solution, MoaMetadata)> {
        // Phase 1: Generate initial completions
        let (completions, phase1_tokens, fallback_used) = Self::generate_initial_completions(
            query,
            system_prompt,
            num_completions,
            client,
            fallback_enabled,
        )
        .await?;

        // Phase 2: Generate critique
        let (critique, phase2_tokens) =
            Self::generate_critique(query, &completions, system_prompt, client).await?;

        // Phase 3: Generate final synthesis
        let (final_answer, phase3_tokens) =
            Self::generate_final_synthesis(query, &completions, &critique, system_prompt, client)
                .await?;

        // Calculate total tokens
        let total_tokens = phase1_tokens + phase2_tokens + phase3_tokens;

        // Create solution from final synthesis
        let reasoning = format!(
            "MOA Aggregation:\n\nCandidates generated: {}\n\nCritique:\n{}\n\nFinal Synthesis:",
            completions.len(),
            critique
        );

        let solution = Solution::new(
            "moa-aggregator".to_string(),
            reasoning,
            final_answer,
            0.5, // Use medium temperature for aggregated solution
            total_tokens,
        );

        let metadata = MoaMetadata {
            total_tokens,
            phase1_tokens,
            phase2_tokens,
            phase3_tokens,
            num_completions: completions.len(),
            fallback_used,
        };

        Ok((solution, metadata))
    }
}


#[cfg(test)]
mod tests;
