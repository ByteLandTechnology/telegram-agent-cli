use crate::app::AppContext;
use crate::automation::filters::build_wait_filter;
use crate::automation::spec::{CleanupStep, ScenarioSpec, ScenarioStep, SendStep, WaitStep};
use crate::commands::message::parse_timeout;
use crate::errors::{Result, TelegramCliError};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;

pub struct ScenarioRunner<'a> {
    context: &'a AppContext,
}

impl<'a> ScenarioRunner<'a> {
    pub fn new(context: &'a AppContext) -> Self {
        Self { context }
    }

    pub async fn run_path(&self, path: &Path) -> Result<i64> {
        let contents = std::fs::read_to_string(path)?;
        let spec: ScenarioSpec = serde_yaml::from_str(&contents).map_err(|error| {
            TelegramCliError::Message(format!("failed to parse scenario: {error}"))
        })?;
        self.run_spec(path, &spec).await
    }

    async fn run_spec(&self, path: &Path, spec: &ScenarioSpec) -> Result<i64> {
        let run_id = self
            .context
            .repo
            .create_test_run(path.display().to_string().as_str())?;
        let mut saved_messages = HashMap::<String, i64>::new();

        let result = async {
            for step in &spec.steps {
                match step {
                    ScenarioStep::Send { send: step } => {
                        let sent = self.execute_send(spec, step).await?;
                        if let Some(key) = &step.save_as {
                            saved_messages.insert(key.clone(), sent.message_id);
                        }
                        self.context.repo.append_run_event(
                            run_id,
                            "send",
                            &json!({
                                "peer_id": sent.peer_id,
                                "message_id": sent.message_id,
                                "text": sent.text,
                            }),
                        )?;
                    }
                    ScenarioStep::Wait { wait: step } => {
                        let message = self.execute_wait(spec, step).await?;
                        self.context.repo.append_run_event(
                            run_id,
                            "wait",
                            &json!({
                                "peer_id": message.peer_id,
                                "message_id": message.message_id,
                                "text": message.text,
                            }),
                        )?;
                    }
                    ScenarioStep::Cleanup { cleanup: step } => {
                        self.execute_cleanup(spec, step, &saved_messages).await?;
                        self.context.repo.append_run_event(
                            run_id,
                            "cleanup",
                            &json!({
                                "chat": step.chat,
                                "saved_message": step.saved_message,
                            }),
                        )?;
                    }
                }
            }

            Ok::<(), TelegramCliError>(())
        }
        .await;

        match result {
            Ok(()) => {
                self.context.repo.finish_test_run(run_id, "passed")?;
                Ok(run_id)
            }
            Err(error) => {
                self.context.repo.finish_test_run(run_id, "failed")?;
                Err(error)
            }
        }
    }

    async fn execute_send(
        &self,
        spec: &ScenarioSpec,
        step: &SendStep,
    ) -> Result<crate::telegram::SentMessage> {
        let account = resolve_account_ref(spec, &step.as_ref);
        let target = resolve_target_ref(spec, &step.to);
        self.context.require_account(account)?;
        let peer = self.context.resolve_peer(account, target).await?;
        self.context
            .telegram
            .send_text(account, peer.peer_id, &step.text, None, None)
            .await
    }

    async fn execute_wait(
        &self,
        spec: &ScenarioSpec,
        step: &WaitStep,
    ) -> Result<crate::telegram::IncomingMessage> {
        let account = resolve_account_ref(spec, &step.as_ref);
        let chat = resolve_target_ref(spec, &step.chat);
        self.context.require_account(account)?;
        let peer = self.context.resolve_peer(account, chat).await?;
        let filter = build_wait_filter(step);
        self.context
            .telegram
            .wait_for_message(
                account,
                peer.peer_id,
                &filter,
                parse_timeout(&step.timeout)?,
            )
            .await
    }

    async fn execute_cleanup(
        &self,
        spec: &ScenarioSpec,
        step: &CleanupStep,
        saved_messages: &HashMap<String, i64>,
    ) -> Result<()> {
        let account = resolve_account_ref(spec, &step.as_ref);
        let chat = resolve_target_ref(spec, &step.chat);
        let message_id = saved_messages
            .get(&step.saved_message)
            .copied()
            .ok_or_else(|| {
                TelegramCliError::Message(format!(
                    "saved message {} was not found in runtime context",
                    step.saved_message
                ))
            })?;

        self.context.require_account(account)?;
        let peer = self.context.resolve_peer(account, chat).await?;
        self.context
            .telegram
            .delete_message(account, peer.peer_id, message_id)
            .await
    }
}

fn resolve_account_ref<'a>(spec: &'a ScenarioSpec, alias_or_name: &'a str) -> &'a str {
    spec.accounts
        .get(alias_or_name)
        .map(|value| value.as_str())
        .unwrap_or(alias_or_name)
}

fn resolve_target_ref<'a>(spec: &'a ScenarioSpec, alias_or_query: &'a str) -> &'a str {
    spec.targets
        .get(alias_or_query)
        .map(|value| value.as_str())
        .unwrap_or(alias_or_query)
}
