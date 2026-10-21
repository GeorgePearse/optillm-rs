//! Prompts from upstream `optillm/plugins/deep_research/research_engine.py`
//! (Apache-2.0, see NOTICE), converted verbatim by `parity/tools/py2rs_prompt.py`.

/// `completion_note` (upstream line 370).
pub fn completion_note(original_query: &str) -> String {
    format!(
        concat!(
            "\n",
            "            \n",
            "## Note on Report Completion\n",
            "\n",
            "This research report represents the findings gathered during the available research time. While comprehensive coverage was the goal, some areas may require additional investigation for complete analysis.\n",
            "\n",
            "For more detailed information on specific aspects of {p0}, additional focused research sessions may be beneficial.\n",
        ),
        p0 = original_query
    )
}

/// `decomposition` (upstream line 404).
pub fn decomposition(initial_query: &str) -> String {
    format!(
        concat!(
            "\n",
            "        You are a research assistant. Given a complex query, break it down into 3-5 focused sub-queries that would help gather comprehensive information.\n",
            "        \n",
            "        Original query: {p0}\n",
            "        \n",
            "        Provide sub-queries in this format:\n",
            "        1. [specific focused question]\n",
            "        2. [specific focused question]\n",
            "        3. [specific focused question]\n",
            "        ...\n",
            "        \n",
            "        Make each sub-query specific and searchable. Focus on different aspects of the main topic.\n",
            "        ",
        ),
        p0 = initial_query
    )
}

/// `preliminary_draft` (upstream line 617).
pub fn preliminary_draft(initial_query: &str) -> String {
    format!(
        concat!(
            "\n",
            "        Generate a preliminary research report structure for the following query using your internal knowledge.\n",
            "        This will serve as an evolving draft that gets refined through iterative research.\n",
            "        \n",
            "        Query: {p0}\n",
            "        \n",
            "        Create a structured report with:\n",
            "        1. Title and Executive Summary (brief)\n",
            "        2. Introduction and Background (what you know)\n",
            "        3. Key Areas to Explore (identify knowledge gaps)\n",
            "        4. Preliminary Findings (from internal knowledge)\n",
            "        5. Research Questions for Investigation\n",
            "        6. Conclusion (preliminary thoughts)\n",
            "        \n",
            "        IMPORTANT: You MUST mark multiple areas that need external research with [NEEDS RESEARCH] tags.\n",
            "        Every claim that would benefit from external evidence should have [SOURCE NEEDED].\n",
            "        This is a preliminary draft - it should have many gaps for iterative improvement.\n",
            "        \n",
            "        Example of proper marking:\n",
            "        - \"Recent studies show [SOURCE NEEDED] that quantum computing...\"\n",
            "        - \"The economic impact [NEEDS RESEARCH: current market data] is significant...\"\n",
            "        - \"Historical context [NEEDS RESEARCH: specific timeline and events] shows...\"\n",
            "        \n",
            "        Include AT LEAST 5-10 [NEEDS RESEARCH] or [SOURCE NEEDED] tags throughout the draft.\n",
            "        Be explicit about what you don't know and what needs external validation.\n",
            "        ",
        ),
        p0 = initial_query
    )
}

/// `gap_analysis` (upstream line 669).
pub fn gap_analysis(original_query: &str, current_draft: &str) -> String {
    format!(
        concat!(
            "\n",
            "        Analyze the following research draft to identify specific gaps and areas that need external research.\n",
            "        Be thorough and aggressive in finding areas for improvement - even good drafts can be enhanced.\n",
            "        \n",
            "        Original Query: {p0}\n",
            "        \n",
            "        Current Draft:\n",
            "        {p1}\n",
            "        \n",
            "        CRITICAL ANALYSIS REQUIRED:\n",
            "        1. MANDATORY: Find ALL [NEEDS RESEARCH], [SOURCE NEEDED], [CITATION NEEDED] tags\n",
            "        2. Identify claims lacking evidence (even if not explicitly marked)\n",
            "        3. Find areas that could benefit from recent data or statistics\n",
            "        4. Spot generalizations that need specific examples\n",
            "        5. Locate outdated information or areas needing current updates\n",
            "        6. Identify missing perspectives or counterarguments\n",
            "        \n",
            "        For each gap you identify, provide:\n",
            "        1. SECTION: Which section has the gap\n",
            "        2. GAP_TYPE: [PLACEHOLDER_TAG, MISSING_INFO, OUTDATED_INFO, NEEDS_EVIDENCE, LACKS_DEPTH, NEEDS_EXAMPLES, MISSING_PERSPECTIVE]\n",
            "        3. SPECIFIC_NEED: Exactly what information is needed\n",
            "        4. SEARCH_QUERY: A specific, targeted search query to address this gap\n",
            "        5. PRIORITY: [HIGH, MEDIUM, LOW] - HIGH for placeholder tags and critical missing info\n",
            "        \n",
            "        Format each gap as:\n",
            "        GAP_ID: [number]\n",
            "        SECTION: [section name]\n",
            "        GAP_TYPE: [type]\n",
            "        SPECIFIC_NEED: [what's missing]\n",
            "        SEARCH_QUERY: [search query to find this info]\n",
            "        PRIORITY: [priority level]\n",
            "        \n",
            "        IMPORTANT: Identify AT LEAST 3-8 gaps. Be critical and thorough.\n",
            "        Even well-written sections can benefit from additional evidence, examples, or perspectives.\n",
            "        Push for depth, accuracy, and comprehensiveness in the research.\n",
            "        ",
        ),
        p0 = original_query, p1 = current_draft
    )
}

/// `denoising` (upstream line 813).
pub fn denoising(
    original_query: &str,
    current_draft: &str,
    retrieval_content: &str,
    citation_context: &str,
) -> String {
    format!(
        concat!(
            "\n",
            "        You are performing a denoising step in a research diffusion process.\n",
            "\n",
            "        TASK: Integrate new retrieved information with the existing draft to reduce \"noise\" (gaps, inaccuracies, incompleteness).\n",
            "\n",
            "        Original Query: {p0}\n",
            "\n",
            "        Current Draft:\n",
            "        {p1}\n",
            "\n",
            "        New Retrieved Information:\n",
            "        {p2}\n",
            "        {p3}\n",
            "\n",
            "        CRITICAL CITATION REQUIREMENTS:\n",
            "        1. EVERY factual claim, statistic, finding, or piece of evidence MUST be cited using [1], [2], etc.\n",
            "        2. Multiple related claims from the same source can share a citation [3]\n",
            "        3. Claims from different sources should have multiple citations like [1,4,7]\n",
            "        4. Direct quotes MUST have citations immediately after the closing quote\n",
            "        5. When integrating new information, ALWAYS add appropriate citations from the available sources\n",
            "        6. Uncited claims will be considered incomplete and must be fixed\n",
            "\n",
            "        DENOISING INSTRUCTIONS:\n",
            "        1. Identify where the new information fills gaps marked with [NEEDS RESEARCH] or [SOURCE NEEDED]\n",
            "        2. Replace placeholder content with specific, detailed information from the retrieved content\n",
            "        3. Add proper citations for ALL new information using the citation numbers shown above\n",
            "        4. Resolve any conflicts between new and existing information\n",
            "        5. Maintain the overall structure and coherence of the draft\n",
            "        6. Enhance depth and accuracy without losing existing valuable insights\n",
            "        7. Mark any remaining research needs with [NEEDS RESEARCH]\n",
            "        8. Review the draft and add missing citations to any uncited factual claims\n",
            "\n",
            "        QUALITY CHECK: Before returning, verify that the majority of substantive claims have citations.\n",
            "        Aim for at least 70% of factual statements to be properly cited.\n",
            "\n",
            "        Return the improved draft with integrated information and comprehensive citations.\n",
            "        ",
        ),
        p0 = original_query, p1 = current_draft, p2 = retrieval_content, p3 = citation_context
    )
}

/// `quality_evaluation` (upstream line 876).
pub fn quality_evaluation(original_query: &str, previous_draft: &str, draft: &str) -> String {
    format!(
        concat!(
            "\n",
            "        Evaluate the research draft quality improvement.\n",
            "        \n",
            "        Original Query: {p0}\n",
            "        \n",
            "        Previous Draft:\n",
            "        {p1}\n",
            "        \n",
            "        Current Draft:\n",
            "        {p2}\n",
            "        \n",
            "        Rate the following aspects from 0.0 to 1.0:\n",
            "        \n",
            "        COMPLETENESS: How well does the current draft address all aspects of the query?\n",
            "        ACCURACY: How accurate and reliable is the information?\n",
            "        DEPTH: How detailed and comprehensive is the analysis?\n",
            "        COHERENCE: How well-structured and logically organized is the draft?\n",
            "        CITATIONS: How well are sources cited and integrated?\n",
            "        IMPROVEMENT: How much better is this draft compared to the previous version?\n",
            "        \n",
            "        Respond ONLY with:\n",
            "        COMPLETENESS: [score]\n",
            "        ACCURACY: [score]\n",
            "        DEPTH: [score]\n",
            "        COHERENCE: [score]\n",
            "        CITATIONS: [score]\n",
            "        IMPROVEMENT: [score]\n",
            "        ",
        ),
        p0 = original_query, p1 = previous_draft, p2 = draft
    )
}

/// `finalization` (upstream line 1117).
pub fn finalization(original_query: &str, final_draft: &str, citation_context: &str) -> String {
    format!(
        concat!(
            "\n",
            "        Apply final polishing to this research report. This is the last step in the TTD-DR diffusion process.\n",
            "\n",
            "        Original Query: {p0}\n",
            "\n",
            "        Current Draft:\n",
            "        {p1}\n",
            "        {p2}\n",
            "\n",
            "        FINALIZATION TASKS:\n",
            "        1. Ensure professional academic formatting with clear sections\n",
            "        2. **CRITICAL**: Verify all citations are properly formatted as [1], [2], etc. and ADD MISSING CITATIONS\n",
            "        3. Add citations to any factual claims, statistics, or findings that lack them\n",
            "        4. Add a compelling title and executive summary\n",
            "        5. Ensure smooth transitions between sections\n",
            "        6. Add conclusion that directly addresses the original query\n",
            "        7. **CRITICAL**: Remove ALL [NEEDS RESEARCH], [SOURCE NEEDED], and similar placeholder tags\n",
            "        8. Replace any remaining placeholders with actual content or remove incomplete sections\n",
            "        9. Polish language and style for clarity and impact\n",
            "\n",
            "        **CRITICAL CITATION REQUIREMENTS**:\n",
            "        - Every major factual claim MUST have a citation\n",
            "        - Statistics, data points, and research findings MUST be cited\n",
            "        - If information came from research, it needs a citation from the available sources above\n",
            "        - Aim for at least 60-70% of substantive claims to have proper citations\n",
            "        - Remove or rephrase claims that cannot be supported with available citations\n",
            "\n",
            "        **CRITICAL QUALITY REQUIREMENTS**:\n",
            "        - The final report must NOT contain ANY placeholder tags: [NEEDS RESEARCH], [SOURCE NEEDED], [Placeholder for...], etc.\n",
            "        - Remove incomplete \"Research Questions for Investigation\" sections with unanswered questions\n",
            "        - Do not include citation placeholders like \"[1] [Placeholder for specific research citation]\"\n",
            "        - If sections are incomplete, either complete them with available information or remove them entirely\n",
            "        - Ensure all statements are backed by available evidence or are clearly marked as preliminary findings\n",
            "        - The report must be publication-ready with no incomplete elements\n",
            "        - DO NOT create a References section - it will be added automatically\n",
            "\n",
            "        Return the final polished research report with comprehensive citations.\n",
            "        ",
        ),
        p0 = original_query, p1 = final_draft, p2 = citation_context
    )
}
