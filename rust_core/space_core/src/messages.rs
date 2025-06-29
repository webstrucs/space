// space/rust_core/space_core/src/messages.rs

use anyhow::{Result, Context};
use std::time::{SystemTime, UNIX_EPOCH};
#[allow(unused_imports)] // <--- CORREÇÃO: Suprimir warning de import não utilizado aqui
use std::collections::HashMap; // Usado em testes unitários para ComponentConfig

use prost::Message as ProstMessage;

include!(concat!(env!("OUT_DIR"), "/space_core.messages.rs"));

impl Message {
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.encode(&mut buf)
            .context("Falha ao serializar Message com prost")?;
        Ok(buf)
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self> {
        Message::decode(bytes)
            .context("Falha ao desserializar bytes em Message com prost")
    }

    pub fn new_start_component_command(name: String) -> Self {
        Message {
            message_type: Some(message::MessageType::Command(ControlCommand {
                command_type: Some(control_command::CommandType::StartComponent(name)),
            })),
        }
    }

    #[allow(dead_code)]
    pub fn new_stop_component_command(name: String) -> Self {
        Message {
            message_type: Some(message::MessageType::Command(ControlCommand {
                command_type: Some(control_command::CommandType::StopComponent(name)),
            })),
        }
    }

    #[allow(dead_code)]
    pub fn new_restart_component_command(name: String) -> Self {
        Message {
            message_type: Some(message::MessageType::Command(ControlCommand {
                command_type: Some(control_command::CommandType::RestartComponent(name)),
            })),
        }
    }

    pub fn new_request_status_command() -> Self {
        Message {
            message_type: Some(message::MessageType::Command(ControlCommand {
                command_type: Some(control_command::CommandType::RequestStatus(true)),
            })),
        }
    }

    #[allow(dead_code)]
    pub fn new_request_component_status_command(name: String) -> Self {
        Message {
            message_type: Some(message::MessageType::Command(ControlCommand {
                command_type: Some(control_command::CommandType::RequestComponentStatus(name)),
            })),
        }
    }

    pub fn new_configure_component_command(name: String, config: ComponentConfig) -> Self {
        Message {
            message_type: Some(message::MessageType::Command(ControlCommand {
                command_type: Some(control_command::CommandType::ConfigureComponent(ConfigureComponentData {
                    name,
                    config: Some(config),
                })),
            })),
        }
    }
    
    #[allow(dead_code)]
    pub fn new_shutdown_command() -> Self {
        Message {
            message_type: Some(message::MessageType::Command(ControlCommand {
                command_type: Some(control_command::CommandType::Shutdown(true)),
            })),
        }
    }

    pub fn new_status(
        component_name: String,
        is_running: bool,
        uptime_seconds: u64,
        current_load: f32,
        message: String,
        state: ComponentState,
        metrics: Option<ComponentMetrics>,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Message {
            message_type: Some(message::MessageType::Status(StatusMessage {
                component_name,
                is_running,
                uptime_seconds,
                current_load,
                message,
                timestamp,
                component_state: state as i32,
                metrics,
            })),
        }
    }

    pub fn new_error(
        error_code: u32,
        description: String,
        source_component: String,
        severity: ErrorSeverity,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Message {
            message_type: Some(message::MessageType::Error(ErrorMessage {
                error_code,
                description,
                source_component,
                timestamp,
                severity: severity as i32,
                debug_info: None,
            })),
        }
    }

    pub fn new_heartbeat() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Message {
            message_type: Some(message::MessageType::HeartbeatTimestamp(timestamp)),
        }
    }

    #[allow(dead_code)]
    pub fn is_command(&self) -> bool {
        matches!(self.message_type, Some(message::MessageType::Command(_)))
    }

    #[allow(dead_code)]
    pub fn is_status(&self) -> bool {
        matches!(self.message_type, Some(message::MessageType::Status(_)))
    }

    #[allow(dead_code)]
    pub fn is_error(&self) -> bool {
        matches!(self.message_type, Some(message::MessageType::Error(_)))
    }
}

impl ComponentMetrics {
    #[allow(dead_code)] // <--- CORREÇÃO: Adicionar #[allow(dead_code)] aqui
    pub fn basic(cpu_usage: f32, memory_usage_mb: u64, active_connections: u32) -> Self {
        Self {
            cpu_usage,
            memory_usage_mb,
            active_connections,
            total_requests: 0,
            error_count: 0,
            requests_per_second: 0.0,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protobuf_command_serialization_deserialization() {
        let original_command = Message::new_start_component_command("http_server".to_string());

        let serialized_data = original_command.serialize().expect("Failed to serialize command");
        assert!(!serialized_data.is_empty());

        let deserialized_message = Message::deserialize(&serialized_data).expect("Failed to deserialize command");

        if let Some(message::MessageType::Command(cmd)) = deserialized_message.message_type {
            if let Some(control_command::CommandType::StartComponent(comp_name)) = cmd.command_type {
                assert_eq!(comp_name, "http_server");
            } else {
                panic!("Deserialized command is not StartComponent");
            }
        } else {
            panic!("Deserialized message is not a Command");
        }
    }

    #[test]
    fn test_protobuf_status_serialization_deserialization() {
        let original_status = Message::new_status(
            "packet_processor".to_string(),
            true,
            3600,
            0.75,
            "All systems nominal".to_string(),
            ComponentState::Running,
            Some(ComponentMetrics::basic(0.5, 512, 10)),
        );

        let serialized_data = original_status.serialize().expect("Failed to serialize status");
        assert!(!serialized_data.is_empty());

        let deserialized_message = Message::deserialize(&serialized_data).expect("Failed to deserialize status");

        if let Some(message::MessageType::Status(status)) = deserialized_message.message_type {
            assert_eq!(status.component_name, "packet_processor");
            assert_eq!(status.is_running, true);
            assert_eq!(status.uptime_seconds, 3600);
            assert_eq!(status.current_load, 0.75);
            assert_eq!(status.message, "All systems nominal");
            assert_eq!(status.component_state, ComponentState::Running as i32);
            assert!(status.metrics.is_some());
            let metrics = status.metrics.unwrap();
            assert_eq!(metrics.cpu_usage, 0.5);
            assert_eq!(metrics.memory_usage_mb, 512);
            assert_eq!(metrics.active_connections, 10);
        } else {
            panic!("Deserialized message is not a StatusMessage");
        }
    }

    #[test]
    fn test_protobuf_error_message() {
        let error_msg = Message::new_error(
            500,
            "Internal server error".to_string(),
            "http_server".to_string(),
            ErrorSeverity::Error,
        );

        let serialized = error_msg.serialize().expect("Failed to serialize error");
        let deserialized = Message::deserialize(&serialized).expect("Failed to deserialize error");

        assert!(deserialized.is_error());
    }

    #[test]
    fn test_protobuf_heartbeat_message() {
        let heartbeat = Message::new_heartbeat();
        let serialized = heartbeat.serialize().expect("Failed to serialize heartbeat");
        let deserialized = Message::deserialize(&serialized).expect("Failed to deserialize heartbeat");

        assert!(matches!(deserialized.message_type, Some(message::MessageType::HeartbeatTimestamp(_))));
    }

    #[test]
    fn test_protobuf_component_config() {
        let mut settings = HashMap::new();
        settings.insert("port".to_string(), "8080".to_string());
        settings.insert("timeout_ms".to_string(), "5000".to_string());

        let original_config_command = Message::new_configure_component_command(
            "http_server".to_string(),
            ComponentConfig {
                settings,
                log_level: Some("DEBUG".to_string()),
                max_resources: Some(ResourceLimits {
                    max_cpu: Some(0.9),
                    max_memory_mb: Some(1024),
                    max_connections: Some(100),
                }),
            },
        );

        let serialized_data = original_config_command.serialize().expect("Failed to serialize config command");
        assert!(!serialized_data.is_empty());

        let deserialized_message = Message::deserialize(&serialized_data).expect("Failed to deserialize config command");

        if let Some(message::MessageType::Command(cmd)) = deserialized_message.message_type {
            if let Some(control_command::CommandType::ConfigureComponent(config_data)) = cmd.command_type {
                assert_eq!(config_data.name, "http_server");
                let config = config_data.config.unwrap();
                assert_eq!(config.settings.get("port"), Some(&"8080".to_string()));
                assert_eq!(config.settings.get("timeout_ms"), Some(&"5000".to_string()));
                assert_eq!(config.log_level, Some("DEBUG".to_string()));
                assert!(config.max_resources.is_some());
                let limits = config.max_resources.unwrap();
                assert_eq!(limits.max_cpu, Some(0.9));
                assert_eq!(limits.max_memory_mb, Some(1024));
                assert_eq!(limits.max_connections, Some(100));
            } else {
                panic!("Deserialized command is not ConfigureComponent");
            }
        } else {
            panic!("Deserialized message is not a Command");
        }
    }
}
