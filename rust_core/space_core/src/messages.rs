//! Este módulo define as estruturas de dados para comunicação entre as camadas
//! de baixo nível (Rust) e alto nível (Python), utilizando serialização
//! e desserialização binária com `bincode`.

use serde::{Serialize, Deserialize};
use bincode;
use anyhow::{Result, Context};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap; // Importar HashMap explicitamente

// --- Mensagens de Comando (Enviadas da Camada Superior para a Inferior) ---

/// Define os tipos de comandos de controle que a camada de alto nível
/// pode enviar para a camada de baixo nível (Rust).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ControlCommand {
    /// Sinal para iniciar um componente específico pelo seu nome.
    StartComponent(String),
    /// Sinal para parar um componente específico pelo seu nome.
    StopComponent(String),
    /// Sinal para reiniciar um componente.
    RestartComponent(String),
    /// Requisita o status atual de um componente ou do sistema.
    RequestStatus,
    /// Requisita o status de um componente específico.
    RequestComponentStatus(String),
    /// Configura parâmetros de um componente.
    ConfigureComponent { name: String, config: ComponentConfig },
    /// Comando para parar o sistema inteiro.
    Shutdown,
    /// Um comando desconhecido ou inválido.
    Unknown,
}

/// Configuração específica para componentes.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComponentConfig {
    /// Configurações específicas como pares chave-valor.
    pub settings: HashMap<String, String>, // Usar HashMap do std::collections
    /// Nível de log para o componente.
    pub log_level: Option<String>,
    /// Recursos máximos permitidos.
    pub max_resources: Option<ResourceLimits>,
}

/// Limites de recursos para componentes.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResourceLimits {
    /// Uso máximo de CPU (0.0 a 1.0).
    pub max_cpu: Option<f32>,
    /// Uso máximo de memória em MB.
    pub max_memory_mb: Option<u64>,
    /// Número máximo de conexões simultâneas.
    pub max_connections: Option<u32>,
}

// --- Mensagens de Status (Enviadas da Camada Inferior para a Superior) ---

/// Define a estrutura de uma mensagem de status que a camada de baixo nível
/// (Rust) pode enviar de volta para a camada de alto nível.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatusMessage {
    /// Nome do componente ao qual o status se refere.
    pub component_name: String,
    /// Indica se o componente está atualmente em execução.
    pub is_running: bool,
    /// Tempo de atividade do componente em segundos.
    pub uptime_seconds: u64,
    /// Carga atual do componente (ex: uso de CPU, memória, etc.).
    pub current_load: f32,
    /// Mensagem adicional de texto para detalhes ou erros.
    pub message: String,
    /// Timestamp da mensagem.
    pub timestamp: u64,
    /// Estado detalhado do componente.
    pub component_state: ComponentState,
    /// Métricas específicas do componente.
    pub metrics: Option<ComponentMetrics>,
}

/// Estado detalhado de um componente.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ComponentState {
    /// Componente inicializando.
    Starting,
    /// Componente rodando normalmente.
    Running,
    /// Componente parando.
    Stopping,
    /// Componente parado.
    Stopped,
    /// Componente em erro.
    Error(String),
    /// Componente reiniciando.
    Restarting,
    /// Estado desconhecido.
    Unknown,
}

/// Métricas específicas de um componente.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComponentMetrics {
    /// Uso atual de CPU (0.0 a 1.0).
    pub cpu_usage: f32,
    /// Uso atual de memória em MB.
    pub memory_usage_mb: u64,
    /// Número de conexões ativas.
    pub active_connections: u32,
    /// Número total de requisições processadas.
    pub total_requests: u64,
    /// Número de erros registrados.
    pub error_count: u64,
    /// Taxa de requisições por segundo.
    pub requests_per_second: f32,
}

// --- Mensagens de Erro ---

/// Estrutura para mensagens de erro específicas.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ErrorMessage {
    /// Código do erro.
    pub error_code: u32,
    /// Descrição do erro.
    pub description: String,
    /// Componente que gerou o erro.
    pub source_component: String,
    /// Timestamp do erro.
    pub timestamp: u64,
    /// Nível de severidade do erro.
    pub severity: ErrorSeverity,
    /// Informações adicionais para debug.
    pub debug_info: Option<String>,
}

/// Nível de severidade do erro.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ErrorSeverity {
    /// Informação apenas.
    Info,
    /// Aviso.
    Warning,
    /// Erro que pode ser recuperado.
    Error,
    /// Erro crítico que pode causar falha do sistema.
    Critical,
}

// --- Mensagem Genérica ---

/// Um enum genérico que pode encapsular qualquer tipo de mensagem trocada
/// entre as camadas.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    /// Encapsula um comando de controle.
    Command(ControlCommand),
    /// Encapsula uma mensagem de status.
    Status(StatusMessage),
    /// Encapsula uma mensagem de erro.
    Error(ErrorMessage),
    /// Mensagem de heartbeat para manter conexão viva.
    Heartbeat { timestamp: u64 },
    /// Acknowledgment de uma mensagem recebida.
    Ack { message_id: String },
}

// --- Implementação de Serialização e Desserialização ---

#[allow(dead_code)] // Permitir funções não utilizadas se forem parte da API
impl Message {
    /// Serializa uma instância de `Message` para um vetor de bytes usando `bincode`.
    pub fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self)
            .context("Failed to serialize Message with bincode")
    }

    /// Desserializa um slice de bytes para uma instância de `Message` usando `bincode`.
    pub fn deserialize(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes)
            .context("Failed to deserialize bytes into Message with bincode")
    }

    /// Cria uma mensagem de status com timestamp atual.
    /// Agora aceita um Option<ComponentMetrics> para incluir métricas diretamente.
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

        Message::Status(StatusMessage {
            component_name,
            is_running,
            uptime_seconds,
            current_load,
            message,
            timestamp,
            component_state: state,
            metrics,
        })
    }

    /// Cria uma mensagem de erro com timestamp atual.
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

        Message::Error(ErrorMessage {
            error_code,
            description,
            source_component,
            timestamp,
            severity,
            debug_info: None,
        })
    }

    /// Cria uma mensagem de heartbeat.
    pub fn new_heartbeat() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Message::Heartbeat { timestamp }
    }

    /// Verifica se é uma mensagem de comando.
    pub fn is_command(&self) -> bool {
        matches!(self, Message::Command(_))
    }

    /// Verifica se é uma mensagem de status.
    pub fn is_status(&self) -> bool {
        matches!(self, Message::Status(_))
    }

    /// Verifica se é uma mensagem de erro.
    pub fn is_error(&self) -> bool {
        matches!(self, Message::Error(_))
    }
}

// --- Implementações auxiliares ---

#[allow(dead_code)] // Permitir funções não utilizadas se forem parte da API
impl StatusMessage {
    /// Atualiza as métricas do status.
    pub fn with_metrics(mut self, metrics: ComponentMetrics) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Verifica se o componente está saudável.
    pub fn is_healthy(&self) -> bool {
        self.is_running && matches!(self.component_state, ComponentState::Running)
    }
}

#[allow(dead_code)] // Permitir funções não utilizadas se forem parte da API
impl ComponentMetrics {
    /// Cria métricas básicas.
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

    /// Adiciona o total de requisições às métricas.
    pub fn with_total_requests(mut self, total_requests: u64) -> Self {
        self.total_requests = total_requests;
        self
    }

    /// Adiciona a contagem de erros às métricas.
    pub fn with_error_count(mut self, error_count: u64) -> Self {
        self.error_count = error_count;
        self
    }

    /// Adiciona a taxa de requisições por segundo às métricas.
    pub fn with_requests_per_second(mut self, rps: f32) -> Self {
        self.requests_per_second = rps;
        self
    }
}

// --- Testes Unitários ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_serialization_deserialization() {
        let original_command = Message::Command(ControlCommand::StartComponent("http_server".to_string()));

        let serialized_data = original_command.serialize().expect("Failed to serialize command");
        assert!(!serialized_data.is_empty());

        let deserialized_message = Message::deserialize(&serialized_data).expect("Failed to deserialize command");

        if let Message::Command(ControlCommand::StartComponent(comp_name)) = deserialized_message {
            assert_eq!(comp_name, "http_server");
        } else {
            panic!("Deserialized message is not the expected ControlCommand::StartComponent");
        }
    }

    #[test]
    fn test_status_serialization_deserialization() {
        let original_status = Message::new_status(
            "packet_processor".to_string(),
            true,
            3600,
            0.75,
            "All systems nominal".to_string(),
            ComponentState::Running,
            None, // Sem métricas para este teste
        );

        let serialized_data = original_status.serialize().expect("Failed to serialize status");
        assert!(!serialized_data.is_empty());

        let deserialized_message = Message::deserialize(&serialized_data).expect("Failed to deserialize status");

        if let Message::Status(status) = deserialized_message {
            assert_eq!(status.component_name, "packet_processor");
            assert_eq!(status.is_running, true);
            assert_eq!(status.uptime_seconds, 3600);
            assert_eq!(status.current_load, 0.75);
            assert_eq!(status.message, "All systems nominal");
            assert!(matches!(status.component_state, ComponentState::Running));
        } else {
            panic!("Deserialized message is not the expected StatusMessage");
        }
    }

    #[test]
    fn test_error_message() {
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
    fn test_heartbeat_message() {
        let heartbeat = Message::new_heartbeat();
        let serialized = heartbeat.serialize().expect("Failed to serialize heartbeat");
        let deserialized = Message::deserialize(&serialized).expect("Failed to deserialize heartbeat");

        assert!(matches!(deserialized, Message::Heartbeat { .. }));
    }

    #[test]
    fn test_component_config() {
        let mut settings = HashMap::new(); // Use HashMap
        settings.insert("port".to_string(), "8080".to_string());
        settings.insert("timeout_ms".to_string(), "5000".to_string());

        let original_config_command = Message::Command(ControlCommand::ConfigureComponent {
            name: "http_server".to_string(),
            config: ComponentConfig {
                settings,
                log_level: Some("DEBUG".to_string()),
                max_resources: Some(ResourceLimits {
                    max_cpu: Some(0.9),
                    max_memory_mb: Some(1024),
                    max_connections: Some(100),
                }),
            },
        });

        let serialized_data = original_config_command.serialize().expect("Failed to serialize config command");
        assert!(!serialized_data.is_empty());

        let deserialized_message = Message::deserialize(&serialized_data).expect("Failed to deserialize config command");

        if let Message::Command(ControlCommand::ConfigureComponent { name, config }) = deserialized_message {
            assert_eq!(name, "http_server");
            assert_eq!(config.settings.get("port"), Some(&"8080".to_string()));
            assert_eq!(config.settings.get("timeout_ms"), Some(&"5000".to_string()));
            assert_eq!(config.log_level, Some("DEBUG".to_string()));
            assert!(config.max_resources.is_some());
            let limits = config.max_resources.unwrap();
            assert_eq!(limits.max_cpu, Some(0.9));
            assert_eq!(limits.max_memory_mb, Some(1024));
            assert_eq!(limits.max_connections, Some(100));
        } else {
            panic!("Deserialized message is not the expected ControlCommand::ConfigureComponent");
        }
    }
}
