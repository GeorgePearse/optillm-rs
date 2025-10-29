#[cfg(test)]

    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::new("user", "Hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_dialogue_state_creation() {
        let state = DialogueState::new(
            "You are helpful".to_string(),
            vec![],
            "What is 2+2?".to_string(),
        );
        assert_eq!(state.system_prompt, "You are helpful");
        assert_eq!(state.current_query, "What is 2+2?");
        assert!(state.conversation_history.is_empty());
    }

    #[test]
    fn test_mcts_node_creation() {
        let state = DialogueState::new("prompt".to_string(), vec![], "query".to_string());
        let node = MCTSNode::new(state, None);
        assert_eq!(node.visits, 0);
        assert_eq!(node.value, 0.0);
        assert!(node.children.is_empty());
        assert!(node.parent.is_none());
    }

    #[test]
    fn test_mcts_node_with_parent() {
        let state = DialogueState::new("prompt".to_string(), vec![], "query".to_string());
        let node = MCTSNode::new(state, Some(0));
        assert_eq!(node.parent, Some(0));
    }

    #[test]
    fn test_mcts_config_defaults() {
        let config = MCTSConfig::default();
        assert_eq!(config.simulation_depth, 1);
        assert_eq!(config.exploration_weight, 0.2);
        assert_eq!(config.num_simulations, 2);
        assert_eq!(config.num_actions, 3);
        assert_eq!(config.generation_temperature, 1.0);
        assert_eq!(config.evaluation_temperature, 0.1);
        assert_eq!(config.max_history_length, 10);
    }

    #[test]
    fn test_mcts_creation() {
        let config = MCTSConfig::default();
        let mcts = MCTS::new(config);
        assert_eq!(mcts.completion_tokens, 0);
        assert!(mcts.nodes.is_empty());
        assert!(mcts.root_idx.is_none());
    }

    #[test]
    fn test_is_terminal_not_terminal() {
        let config = MCTSConfig::default();
        let mcts = MCTS::new(config);

        let state = DialogueState::new("".to_string(), vec![], "hello".to_string());
        assert!(!mcts.is_terminal(&state));
    }

    #[test]
    fn test_is_terminal_with_goodbye() {
        let config = MCTSConfig::default();
        let mcts = MCTS::new(config);

        let state = DialogueState::new("".to_string(), vec![], "goodbye".to_string());
        assert!(mcts.is_terminal(&state));
    }

    #[test]
    fn test_is_terminal_max_history() {
        let config = MCTSConfig::default();
        let mcts = MCTS::new(config);

        // Create history with more than max_history_length messages
        let mut history = Vec::new();
        for i in 0..15 {
            history.push(Message::new(
                if i % 2 == 0 { "user" } else { "assistant" },
                format!("Message {}", i),
            ));
        }

        let state = DialogueState::new("".to_string(), history, "hello".to_string());
        assert!(mcts.is_terminal(&state));
    }
