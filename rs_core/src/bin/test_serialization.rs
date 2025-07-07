// Conteúdo para: rs_core/src/bin/test_serialization.rs

// Importa as mensagens que acabamos de definir
use rs_core::ipc::protocol::LowLevelMessage;

fn main() {
    println!("--- Teste de Serialização/Desserialização ---");

    // 1. Cria uma instância da nossa mensagem.
    let original_message = LowLevelMessage::Data {
        conn_id: 123,
        data: vec![72, 101, 108, 108, 111], // "Hello" em bytes ASCII
    };
    println!("Mensagem Original: {:?}", original_message);

    // 2. Serializa a mensagem para um vetor de bytes usando bincode.
    let serialized_data = bincode::serialize(&original_message).expect("Falha ao serializar");
    println!("Dados Serializados ({} bytes): {:?}", serialized_data.len(), serialized_data);

    // 3. Desserializa os bytes de volta para a nossa struct.
    let deserialized_message: LowLevelMessage = bincode::deserialize(&serialized_data).expect("Falha ao desserializar");
    println!("Mensagem Desserializada: {:?}", deserialized_message);

    // 4. Verifica se a mensagem original e a desserializada são idênticas.
    assert_eq!(original_message, deserialized_message);

    println!("\n✅ Teste de round-trip concluído com sucesso!");
}