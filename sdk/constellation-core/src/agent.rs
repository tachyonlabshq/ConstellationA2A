//! The core agent module providing [`ConstellationAgent`].
//!
//! This is the primary entry point for interacting with the Constellation A2A system.
//! An agent connects to a Matrix homeserver, joins rooms, registers event handlers,
//! and runs a sync loop to dispatch incoming messages.

use std::sync::Arc;

use matrix_sdk::{
    config::SyncSettings,
    room::Room,
    ruma::{
        api::client::room::create_room::v3::Request as CreateRoomRequest,
        events::room::message::{MessageType, OriginalSyncRoomMessageEvent},
        OwnedUserId, RoomAliasId,
    },
    Client,
};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, warn};

use crate::config::AgentConfig;
use crate::error::{ConstellationError, Result};
use crate::message::{
    parse_mentions, ConstellationMetadata, MentionEvent, Message, MessageEvent, Task, TaskEvent,
    TaskResult,
};
use crate::room::RoomHandle;
use crate::task::TaskManager;

type MentionHandler = Arc<dyn Fn(MentionEvent) + Send + Sync>;
type MessageHandler = Arc<dyn Fn(MessageEvent) + Send + Sync>;
type TaskHandler = Arc<dyn Fn(TaskEvent) + Send + Sync>;

/// The main Constellation agent that wraps a Matrix client and provides
/// high-level APIs for agent-to-agent communication.
///
/// # Lifecycle
///
/// 1. Create with [`ConstellationAgent::new`]
/// 2. Register handlers with [`on_mention`](Self::on_mention), [`on_message`](Self::on_message), [`on_task`](Self::on_task)
/// 3. Call [`connect`](Self::connect) to log in and sync
/// 4. Call [`run`](Self::run) to start the event dispatch loop
/// 5. Call [`disconnect`](Self::disconnect) to shut down gracefully
///
/// # Example
///
/// ```no_run
/// use constellation_core::{AgentConfigBuilder, ConstellationAgent};
///
/// # async fn example() -> constellation_core::Result<()> {
/// let config = AgentConfigBuilder::new()
///     .homeserver_url("http://localhost:6167")
///     .username("agent-researcher")
///     .password("secret")
///     .display_name("Research Agent")
///     .build()?;
///
/// let mut agent = ConstellationAgent::new(config)?;
/// agent.on_mention(|event| {
///     println!("Mentioned by {} in {}: {}", event.sender, event.room_id, event.body);
/// });
/// agent.connect().await?;
/// agent.run().await?;
/// # Ok(())
/// # }
/// ```
pub struct ConstellationAgent {
    config: AgentConfig,
    client: Option<Client>,
    user_id: Option<OwnedUserId>,
    mention_handlers: Arc<Mutex<Vec<MentionHandler>>>,
    message_handlers: Arc<Mutex<Vec<MessageHandler>>>,
    task_handlers: Arc<Mutex<Vec<TaskHandler>>>,
    task_manager: Arc<Mutex<TaskManager>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl std::fmt::Debug for ConstellationAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConstellationAgent")
            .field("config", &self.config)
            .field("connected", &self.client.is_some())
            .finish()
    }
}

impl ConstellationAgent {
    /// Create a new agent from the given configuration.
    ///
    /// Validates the config immediately and returns an error if required fields
    /// (username, password, homeserver_url) are missing.
    pub fn new(config: AgentConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            client: None,
            user_id: None,
            mention_handlers: Arc::new(Mutex::new(Vec::new())),
            message_handlers: Arc::new(Mutex::new(Vec::new())),
            task_handlers: Arc::new(Mutex::new(Vec::new())),
            task_manager: Arc::new(Mutex::new(TaskManager::new())),
            shutdown_tx: None,
        })
    }

    /// Connect to the Matrix homeserver, log in, and perform an initial sync.
    ///
    /// After connecting, the agent will automatically attempt to join any rooms
    /// listed in [`AgentConfig::auto_join_rooms`]. Failures to join individual
    /// rooms are logged as warnings but do not fail the connection.
    pub async fn connect(&mut self) -> Result<()> {
        info!(
            homeserver = %self.config.homeserver_url,
            username = %self.config.username,
            "Connecting to Matrix homeserver"
        );

        let homeserver = url::Url::parse(&self.config.homeserver_url)?;
        let client = Client::builder()
            .homeserver_url(homeserver)
            .build()
            .await
            .map_err(|e| {
                ConstellationError::Connection(format!("failed to build client: {e}"))
            })?;

        // Log in with username/password.
        let mut login = client
            .matrix_auth()
            .login_username(&self.config.username, &self.config.password);
        if let Some(ref device_id) = self.config.device_id {
            login = login.device_id(device_id);
        }
        login
            .initial_device_display_name(
                self.config
                    .display_name
                    .as_deref()
                    .unwrap_or(&self.config.username),
            )
            .send()
            .await?;

        info!("Logged in successfully");

        // Set display name if provided.
        if let Some(ref display_name) = self.config.display_name {
            if let Err(e) = client
                .account()
                .set_display_name(Some(display_name))
                .await
            {
                warn!("Failed to set display name: {e}");
            }
        }

        // Perform initial sync to get room state.
        client.sync_once(SyncSettings::default()).await?;
        info!("Initial sync complete");

        self.user_id = client.user_id().map(|id| id.to_owned());
        self.client = Some(client.clone());

        // Auto-join configured rooms.
        for room_alias in &self.config.auto_join_rooms {
            match self.join_room_inner(&client, room_alias).await {
                Ok(handle) => {
                    info!(room = %room_alias, room_id = %handle.room_id(), "Auto-joined room")
                }
                Err(e) => warn!(room = %room_alias, error = %e, "Failed to auto-join room"),
            }
        }

        Ok(())
    }

    /// Gracefully disconnect from the homeserver.
    ///
    /// Sends a shutdown signal to the sync loop (if running) and logs out
    /// from the Matrix server, invalidating the access token.
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        if let Some(ref client) = self.client {
            info!("Logging out");
            client
                .matrix_auth()
                .logout()
                .await
                .map_err(|e| ConstellationError::Connection(format!("logout failed: {e}")))?;
        }
        self.client = None;
        self.user_id = None;
        Ok(())
    }

    /// Join a room by alias (e.g. `#agents:constellation.local`).
    ///
    /// Returns a [`RoomHandle`] for sending messages and querying room state.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent is not connected, the alias is invalid,
    /// or the server rejects the join request.
    pub async fn join_room(&self, room_alias: &str) -> Result<RoomHandle> {
        let client = self.require_client()?;
        self.join_room_inner(client, room_alias).await
    }

    async fn join_room_inner(&self, client: &Client, room_alias: &str) -> Result<RoomHandle> {
        info!(room = %room_alias, "Joining room");

        let response = client
            .join_room_by_id_or_alias(
                <&RoomAliasId>::try_from(room_alias)?.into(),
                &[],
            )
            .await?;

        let room = client.get_room(response.room_id()).ok_or_else(|| {
            ConstellationError::Room(format!(
                "room {} joined but not found in client state",
                response.room_id()
            ))
        })?;

        Ok(RoomHandle::new(room))
    }

    /// Create a new Matrix room and optionally invite other agents.
    ///
    /// # Parameters
    ///
    /// - `name`: The human-readable room name.
    /// - `invited_agents`: Slice of Matrix user IDs (e.g. `@agent-b:server`) to invite.
    ///
    /// # Errors
    ///
    /// Returns an error if any invited agent ID is malformed, the agent is not
    /// connected, or the server rejects the room creation.
    pub async fn create_room(
        &self,
        name: &str,
        invited_agents: &[&str],
    ) -> Result<RoomHandle> {
        let client = self.require_client()?;
        info!(name = %name, "Creating room");

        let mut request = CreateRoomRequest::new();
        request.name = Some(name.to_string());

        let invite: std::result::Result<Vec<OwnedUserId>, _> = invited_agents
            .iter()
            .map(|a| OwnedUserId::try_from(*a))
            .collect();
        request.invite = invite?;

        let response = client.create_room(request).await?;
        let room = client.get_room(response.room_id()).ok_or_else(|| {
            ConstellationError::Room(format!(
                "room {} created but not found in client state",
                response.room_id()
            ))
        })?;

        Ok(RoomHandle::new(room))
    }

    /// Send a message to a room.
    ///
    /// The message may optionally contain [`ConstellationMetadata`] which will be
    /// embedded as the `ai.constellation.metadata` field in the event content.
    pub async fn send_message(&self, room: &RoomHandle, msg: Message) -> Result<()> {
        room.send_message(&msg).await
    }

    /// Send a message that @-mentions a specific agent by their Matrix user ID.
    ///
    /// The message body is prefixed with an HTML mention link so the target
    /// agent's [`on_mention`](Self::on_mention) handler will fire.
    ///
    /// # Parameters
    ///
    /// - `room`: The room to send the mention in.
    /// - `agent_user_id`: Full Matrix user ID, e.g. `@agent-b:constellation.local`.
    /// - `msg`: The message content to send after the mention.
    pub async fn mention_agent(
        &self,
        room: &RoomHandle,
        agent_user_id: &str,
        msg: Message,
    ) -> Result<()> {
        // Use the localpart as display name if we can't look it up.
        let display_name = agent_user_id.split(':').next().unwrap_or(agent_user_id);
        room.send_mention(agent_user_id, display_name, &msg).await
    }

    /// Register a handler that fires when this agent is @-mentioned in a message.
    ///
    /// Multiple handlers can be registered and will all be called for each mention.
    /// Handlers must be `Send + Sync + 'static` as they execute in the sync loop task.
    ///
    /// **Note:** Register handlers before calling [`run`](Self::run) to ensure
    /// no events are missed.
    pub async fn on_mention(&self, handler: impl Fn(MentionEvent) + Send + Sync + 'static) {
        self.mention_handlers.lock().await.push(Arc::new(handler));
    }

    /// Register a handler for all incoming messages in joined rooms.
    ///
    /// This fires for every text message, including messages that also trigger
    /// mention or task handlers. The handler receives a [`MessageEvent`] with
    /// the full raw event JSON.
    ///
    /// **Note:** Register handlers before calling [`run`](Self::run) to ensure
    /// no events are missed.
    pub async fn on_message(&self, handler: impl Fn(MessageEvent) + Send + Sync + 'static) {
        self.message_handlers.lock().await.push(Arc::new(handler));
    }

    /// Register a handler for structured task events.
    ///
    /// This fires when a message contains `ai.constellation.metadata` with a
    /// valid task definition. The handler receives a [`TaskEvent`] with the
    /// parsed task fields.
    ///
    /// **Note:** Register handlers before calling [`run`](Self::run) to ensure
    /// no events are missed.
    pub async fn on_task(&self, handler: impl Fn(TaskEvent) + Send + Sync + 'static) {
        self.task_handlers.lock().await.push(Arc::new(handler));
    }

    /// Create a task, send it to a room as a message with metadata, and track it locally.
    ///
    /// Returns the task ID which can later be used with [`complete_task`](Self::complete_task).
    pub async fn create_task(&self, room: &RoomHandle, task: Task) -> Result<String> {
        let task_id = task.id.clone();
        info!(task_id = %task_id, task_type = %task.task_type, "Creating task");

        // Track the task locally.
        {
            let mut mgr = self.task_manager.lock().await;
            mgr.create(
                &task_id,
                &task.task_type,
                task.payload.clone(),
                room.room_id(),
            );
        }

        // Send the task as a message with constellation metadata.
        let msg = Message::text(format!("[task:{}] {}", task.task_type, task.id))
            .with_metadata(task.to_metadata());
        room.send_message(&msg).await?;

        Ok(task_id)
    }

    /// Mark a task as completed and send a result message to its originating room.
    ///
    /// # Errors
    ///
    /// Returns [`ConstellationError::Task`] if the task ID is not found in the
    /// local task manager.
    pub async fn complete_task(&self, task_id: &str, result: TaskResult) -> Result<()> {
        let room_id = {
            let mut mgr = self.task_manager.lock().await;
            let record = mgr
                .get(task_id)
                .ok_or_else(|| ConstellationError::Task(format!("task not found: {task_id}")))?;
            let room_id = record.room_id.clone();
            mgr.complete(task_id, result.clone())?;
            room_id
        };

        // Send completion message back to the room.
        let client = self.require_client()?;
        if let Some(room) =
            client.get_room(<&matrix_sdk::ruma::RoomId>::try_from(room_id.as_str())?)
        {
            let handle = RoomHandle::new(room);
            let status_str = match result.status {
                crate::message::TaskStatus::Completed => "completed",
                crate::message::TaskStatus::Failed => "failed",
                _ => "updated",
            };
            let msg = Message::text(format!("[task-result:{status_str}] {task_id}"));
            handle.send_message(&msg).await?;
        }

        Ok(())
    }

    /// Get a locked reference to the internal [`TaskManager`].
    ///
    /// Useful for querying task status or listing pending tasks outside of
    /// the normal handler flow.
    pub async fn task_manager(&self) -> tokio::sync::MutexGuard<'_, TaskManager> {
        self.task_manager.lock().await
    }

    /// Start the sync loop, dispatching incoming events to registered handlers.
    ///
    /// This method blocks until [`disconnect`](Self::disconnect) is called or the
    /// process is interrupted. It continuously polls the Matrix homeserver for new
    /// events and dispatches them to the appropriate handlers.
    ///
    /// The sync loop tracks the `next_batch` token so each iteration only fetches
    /// new events since the last sync.
    ///
    /// # Errors
    ///
    /// Returns an error if the agent is not connected or not logged in.
    /// Individual sync failures are logged and retried after a 5-second delay.
    pub async fn run(&mut self) -> Result<()> {
        let client = self.require_client()?.clone();
        let my_user_id = self
            .user_id
            .clone()
            .ok_or_else(|| ConstellationError::Connection("not logged in".to_string()))?;

        let mention_handlers = self.mention_handlers.clone();
        let message_handlers = self.message_handlers.clone();
        let task_handlers = self.task_handlers.clone();

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        info!("Starting sync loop");

        // Register the event handler on the client.
        client.add_event_handler({
            let my_user_id = my_user_id.clone();
            let mention_handlers = mention_handlers.clone();
            let message_handlers = message_handlers.clone();
            let task_handlers = task_handlers.clone();

            move |event: OriginalSyncRoomMessageEvent, room: Room| {
                let my_user_id = my_user_id.clone();
                let mention_handlers = mention_handlers.clone();
                let message_handlers = message_handlers.clone();
                let task_handlers = task_handlers.clone();

                async move {
                    // Ignore our own messages.
                    if event.sender == my_user_id {
                        return;
                    }

                    let room_id = room.room_id().to_string();
                    let sender = event.sender.to_string();

                    let body = match &event.content.msgtype {
                        MessageType::Text(text) => text.body.clone(),
                        _ => return,
                    };

                    // Extract constellation metadata directly from event content.
                    let content_raw = match serde_json::to_value(&event.content) {
                        Ok(v) => v,
                        Err(e) => {
                            warn!("Failed to serialize event content: {e}");
                            serde_json::Value::Null
                        }
                    };
                    let metadata: Option<ConstellationMetadata> = content_raw
                        .get("ai.constellation.metadata")
                        .and_then(|m| serde_json::from_value(m.clone()).ok());

                    // Build raw event representation for MessageEvent.
                    let raw_event = serde_json::json!({
                        "sender": sender,
                        "room_id": room_id,
                        "content": content_raw,
                        "event_id": event.event_id.to_string(),
                        "origin_server_ts": event.origin_server_ts.get(),
                    });

                    // --- Dispatch to message handlers ---
                    {
                        let handlers = message_handlers.lock().await;
                        let msg_event = MessageEvent {
                            sender: sender.clone(),
                            room_id: room_id.clone(),
                            body: body.clone(),
                            raw_event: raw_event.clone(),
                        };
                        for handler in handlers.iter() {
                            handler(msg_event.clone());
                        }
                    }

                    // --- Dispatch to mention handlers if this agent is mentioned ---
                    let mentions = parse_mentions(&body);
                    let my_id_str = my_user_id.to_string();
                    if mentions.iter().any(|m| m == &my_id_str) {
                        let handlers = mention_handlers.lock().await;
                        let mention_event = MentionEvent {
                            sender: sender.clone(),
                            room_id: room_id.clone(),
                            body: body.clone(),
                            metadata: metadata.clone(),
                            mentioned_agents: mentions.clone(),
                        };
                        for handler in handlers.iter() {
                            handler(mention_event.clone());
                        }
                    }

                    // --- Dispatch to task handlers if constellation metadata is present ---
                    if let Some(ref meta) = metadata {
                        let handlers = task_handlers.lock().await;
                        let task_event = TaskEvent {
                            sender: sender.clone(),
                            room_id: room_id.clone(),
                            task_id: meta.task_id.clone(),
                            task_type: meta.task_type.clone(),
                            payload: meta.payload.clone(),
                            priority: meta.priority,
                        };
                        for handler in handlers.iter() {
                            handler(task_event.clone());
                        }
                    }
                }
            }
        });

        // Run the sync loop until shutdown, tracking the sync token between iterations.
        let mut sync_settings = SyncSettings::default();
        tokio::select! {
            _ = async {
                loop {
                    match client.sync_once(sync_settings.clone()).await {
                        Ok(response) => {
                            debug!("Sync tick: {} joined rooms", response.rooms.join.len());
                            // Forward the next_batch token so we only get new events.
                            sync_settings = sync_settings.token(response.next_batch);
                        }
                        Err(e) => {
                            error!("Sync error: {e}");
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        }
                    }
                }
            } => {}
            _ = shutdown_rx.recv() => {
                info!("Shutdown signal received, stopping sync loop");
            }
        }

        Ok(())
    }

    /// Get a reference to the underlying [`matrix_sdk::Client`], if connected.
    pub fn client(&self) -> Option<&Client> {
        self.client.as_ref()
    }

    /// Get this agent's Matrix user ID. Available after [`connect`](Self::connect).
    pub fn user_id(&self) -> Option<&OwnedUserId> {
        self.user_id.as_ref()
    }

    fn require_client(&self) -> Result<&Client> {
        self.client
            .as_ref()
            .ok_or_else(|| ConstellationError::Connection("not connected".to_string()))
    }
}
