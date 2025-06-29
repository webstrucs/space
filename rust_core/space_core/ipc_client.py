# # space/rust_core/space_core/ipc_client.py

import asyncio
import os
import struct
import sys
import logging

# Configurar logging para ver as mensagens detalhadas
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

# Importar as classes Protobuf geradas
# Certifique-se de que messages_pb2.py está no mesmo diretório ou no PYTHONPATH
try:
    from messages_pb2 import Message, ControlCommand, StatusMessage, ErrorMessage, ComponentState, ErrorSeverity
except ImportError:
    logger.error("Erro ao importar messages_pb2.py. Certifique-se de que foi gerado e está no PYTHONPATH.")
    logger.info("Para gerar: navegue até o diretório raiz do seu projeto Rust e execute:")
    logger.info("protoc --python_out=. --proto_path=proto/ proto/messages.proto")
    sys.exit(1)

# Caminho para o socket de domínio Unix (deve corresponder ao do Rust)
IPC_SOCKET_PATH = "/tmp/space_core_ipc.sock"

async def send_message(writer: asyncio.StreamWriter, message: Message):
    """Envia uma mensagem Protobuf para o servidor Rust."""
    serialized_data = message.SerializePartialToString() # Usa SerializePartialToString
    message_size = len(serialized_data)

    # Envia o tamanho da mensagem (u32, little-endian)
    writer.write(struct.pack("<I", message_size))
    # Envia a mensagem serializada
    writer.write(serialized_data)
    await writer.drain() # Garante que os dados foram enviados

    logger.info(f"Mensagem enviada (tamanho: {message_size} bytes): {message.WhichOneof('message_type')}")


async def receive_message(reader: asyncio.StreamReader) -> Message | None:
    """Recebe uma mensagem Protobuf do servidor Rust."""
    try:
        # Lê os 4 bytes do cabeçalho (tamanho da mensagem)
        header = await reader.readexactly(4)
        message_size = struct.unpack("<I", header)[0]
        
        if message_size == 0:
            logger.debug("Mensagem recebida com tamanho zero, ignorando.")
            return None

        logger.debug(f"Recebido cabeçalho de mensagem com tamanho: {message_size} bytes")

        # Lê a mensagem completa
        serialized_data = await reader.readexactly(message_size)
        
        # Desserializa a mensagem
        message = Message()
        message.ParseFromString(serialized_data)
        
        logger.info(f"Mensagem recebida: {message.WhichOneof('message_type')}")
        return message

    except asyncio.IncompleteReadError as e:
        logger.warning(f"Conexão fechada pelo servidor ou leitura incompleta: {e}")
        return None
    except Exception as e:
        logger.error(f"Erro ao receber ou desserializar mensagem: {e}")
        return None


async def main():
    logger.info(f"Conectando ao servidor IPC Rust em: {IPC_SOCKET_PATH}")
    reader = None
    writer = None
    try:
        # Conectar ao socket de domínio Unix
        reader, writer = await asyncio.open_unix_connection(IPC_SOCKET_PATH)
        logger.info("Conexão IPC estabelecida com sucesso!")

        # --- Exemplo: Enviar um comando "StartComponent" ---
        logger.info("Enviando comando: StartComponent('http_server')")
        cmd_msg = Message(command=ControlCommand(start_component="http_server"))
        await send_message(writer, cmd_msg)

        logger.info("Enviando comando: RequestStatus")
        cmd_msg = Message(command=ControlCommand(request_status=True))
        await send_message(writer, cmd_msg)

        logger.info("Enviando comando: StartComponent('packet_processor')")
        cmd_msg = Message(command=ControlCommand(start_component="packet_processor"))
        await send_message(writer, cmd_msg)

        # --- Loop para receber mensagens do servidor Rust ---
        logger.info("Aguardando mensagens do servidor Rust...")
        while True:
            message = await receive_message(reader)
            if message is None:
                break # Conexão fechada ou erro irrecuperável

            # Processar o tipo de mensagem recebida
            message_type = message.WhichOneof('message_type')
            if message_type == 'status':
                status = message.status
                logger.info(f"  [STATUS] Componente: {status.component_name}, Rodando: {status.is_running}, "
                            f"Carga: {status.current_load:.2f}, Mensagem: {status.message}, "
                            f"Estado: {ComponentState.Name(status.component_state)}")
                if status.HasField('metrics'):
                    metrics = status.metrics
                    logger.info(f"    Métricas: CPU={metrics.cpu_usage:.2f}, Memória={metrics.memory_usage_mb}MB, "
                                f"Conexões={metrics.active_connections}, Requisições={metrics.total_requests}, "
                                f"Erros={metrics.error_count}, RPS={metrics.requests_per_second:.2f}")
            elif message_type == 'error':
                error_msg = message.error
                logger.error(f"  [ERRO] Origem: {error_msg.source_component}, Código: {error_msg.error_code}, "
                             f"Severidade: {ErrorSeverity.Name(error_msg.severity)}, Descrição: {error_msg.description}")
            elif message_type == 'heartbeat_timestamp':
                logger.debug(f"  [HEARTBEAT] Recebido heartbeat em: {message.heartbeat_timestamp}")
            elif message_type == 'ack_message_id':
                logger.debug(f"  [ACK] Acknowledge para ID: {message.ack_message_id}")
            elif message_type == 'command':
                # Comandos não devem ser recebidos do servidor Rust, isso é inesperado
                logger.warning(f"  [AVISO] Comando inesperado recebido do servidor Rust: {message.command.WhichOneof('command_type')}")
            else:
                logger.warning(f"  [AVISO] Tipo de mensagem desconhecido: {message_type}")

            await asyncio.sleep(0.5) # Pequena pausa para não sobrecarregar o loop

    except FileNotFoundError:
        logger.error(f"Erro: Socket IPC não encontrado em {IPC_SOCKET_PATH}. "
                     "Certifique-se de que o servidor Rust está rodando e criou o socket.")
    except ConnectionRefusedError:
        logger.error("Erro: Conexão IPC recusada. O servidor Rust pode não estar escutando ou o socket está corrompido.")
    except Exception as e:
        logger.error(f"Ocorreu um erro inesperado: {e}", exc_info=True)
    finally:
        if writer:
            logger.info("Fechando conexão IPC.")
            writer.close()
            await writer.wait_closed()


if __name__ == "__main__":
    # Remove o arquivo .pyc se existir, para evitar problemas de cache ao regenerar
    if os.path.exists("messages_pb2.pyc"):
        os.remove("messages_pb2.pyc")
    
    # Executa a função assíncrona principal
    asyncio.run(main())

