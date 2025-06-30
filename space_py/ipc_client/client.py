import asyncio
import os
import struct
from typing import AsyncGenerator, Optional

# Importar as classes geradas pelo Protobuf
from space_py.ipc_client import messages_pb2

# Caminho para o socket de domínio Unix do servidor Rust
IPC_SOCKET_PATH = "/tmp/space_core_ipc.sock"

class IPCClient:
    """
    Cliente IPC assíncrono para comunicação com o servidor Space Core Rust via Unix Domain Socket.
    Permite enviar comandos e receber mensagens de status/erros.
    """

    def __init__(self, socket_path: str = IPC_SOCKET_PATH):
        self.socket_path = socket_path
        self.reader: Optional[asyncio.StreamReader] = None
        self.writer: Optional[asyncio.StreamWriter] = None
        self.connected = asyncio.Event() # Evento para sinalizar o estado da conexão
        self.shutdown_event = asyncio.Event() # Evento para sinalizar o desligamento gracioso

    async def connect(self):
        """Tenta conectar ao servidor IPC."""
        while not self.shutdown_event.is_set():
            try:
                info(f"Tentando conectar ao servidor IPC em {self.socket_path}...")
                self.reader, self.writer = await asyncio.open_unix_connection(self.socket_path)
                self.connected.set() # Sinaliza que a conexão foi estabelecida
                info("Conectado com sucesso ao servidor IPC.")
                break # Sai do loop se a conexão for bem-sucedida
            except FileNotFoundError:
                error(f"Socket IPC não encontrado em {self.socket_path}. O servidor Rust está rodando?")
            except ConnectionRefusedError:
                error("Conexão IPC recusada. O servidor Rust está pronto para aceitar conexões?")
            except Exception as e:
                error(f"Erro inesperado ao tentar conectar ao IPC: {e}")

            if not self.shutdown_event.is_set():
                await asyncio.sleep(2) # Espera antes de tentar novamente

        if self.shutdown_event.is_set():
            info("Tentativa de conexão abortada devido ao sinal de desligamento.")
            self.connected.clear() # Garante que o estado seja desconectado
            raise asyncio.CancelledError("Conexão cancelada durante o desligamento.")


    async def disconnect(self):
        """Desconecta do servidor IPC."""
        if self.writer:
            info("Fechando conexão IPC...")
            self.writer.close()
            await self.writer.wait_closed()
            self.reader = None
            self.writer = None
        self.connected.clear() # Limpa o evento de conexão
        info("Conexão IPC fechada.")

    async def send_message(self, message: messages_pb2.Message):
        """
        Envia uma mensagem Protobuf para o servidor IPC.
        A mensagem é prefixada com seu tamanho (4 bytes, little-endian).
        """
        if not self.connected.is_set() or not self.writer:
            error("Não conectado ao servidor IPC. Não é possível enviar a mensagem.")
            return

        try:
            serialized_data = message.SerializeToString()
            message_size = len(serialized_data)

            # Empacota o tamanho da mensagem como um u32 little-endian
            header = struct.pack('<I', message_size)

            debug(f"Enviando mensagem IPC de {message_size} bytes: {message.DESCRIPTOR.full_name}")
            self.writer.write(header + serialized_data)
            await self.writer.drain() # Garante que os dados foram enviados para o socket
            info(f"Mensagem enviada com sucesso: {message.DESCRIPTOR.full_name}")
        except ConnectionResetError:
            error("A conexão IPC foi redefinida pelo peer (servidor Rust desconectou).")
            self.connected.clear() # A conexão foi perdida
        except Exception as e:
            error(f"Erro ao enviar mensagem IPC: {e}")
            self.connected.clear() # Assume que a conexão foi perdida

    async def receive_messages(self) -> AsyncGenerator[messages_pb2.Message, None]:
        """
        Gera mensagens Protobuf recebidas do servidor IPC.
        Espera pelo tamanho da mensagem (4 bytes) e depois pelos dados da mensagem.
        """
        await self.connected.wait() # Espera até que a conexão seja estabelecida

        while self.connected.is_set() and not self.shutdown_event.is_set() and self.reader:
            try:
                # Lê o cabeçalho de 4 bytes (tamanho da mensagem)
                header_data = await self.reader.readexactly(4)
                message_size = struct.unpack('<I', header_data)[0]

                if message_size == 0:
                    debug("Recebido cabeçalho de mensagem IPC com tamanho 0, ignorando.")
                    continue

                # Lê os dados da mensagem
                message_data = await self.reader.readexactly(message_size)
                
                # Desserializa a mensagem
                message = messages_pb2.Message()
                message.ParseFromString(message_data)
                
                info(f"Mensagem IPC recebida: {message.DESCRIPTOR.full_name}")
                yield message # Retorna a mensagem para o consumidor

            except asyncio.IncompleteReadError as e:
                info(f"Conexão IPC fechada pelo servidor (IncompleteRead): {e}")
                self.connected.clear()
                break
            except ConnectionResetError:
                error("A conexão IPC foi redefinida pelo peer (servidor Rust desconectou).")
                self.connected.clear()
                break
            except Exception as e:
                error(f"Erro ao receber mensagem IPC: {e}")
                self.connected.clear()
                break
        info("Loop de recebimento de mensagens IPC encerrado.")

    async def receive_messages_consumer(self):
        """Consome e imprime as mensagens recebidas do servidor."""
        async for message in self.receive_messages():
            if message.HasField('status'):
                status_msg = message.status
                # Corrigido: Acessando ComponentState diretamente do messages_pb2
                info(f"STATUS RECEBIDO: Componente='{status_msg.component_name}', Rodando={status_msg.is_running}, Estado={messages_pb2.ComponentState.Name(status_msg.component_state)}")
                if status_msg.HasField('metrics'):
                    metrics = status_msg.metrics
                    info(f"  Métricas: CPU={metrics.cpu_usage:.2f}, Memória={metrics.memory_usage_mb}MB, Conexões Ativas={metrics.active_connections}")
            elif message.HasField('error'):
                error_msg = message.error
                # Corrigido: Acessando ErrorSeverity diretamente do messages_pb2
                error(f"ERRO RECEBIDO: Origem='{error_msg.source_component}', Código={error_msg.error_code}, Severidade={messages_pb2.ErrorSeverity.Name(error_msg.severity)}, Descrição='{error_msg.description}'")
            elif message.HasField('heartbeat_timestamp'):
                debug(f"HEARTBEAT RECEBIDO: Timestamp={message.heartbeat_timestamp}")
            else:
                warn(f"Mensagem IPC desconhecida recebida: {message}")

    async def shutdown(self):
        """Sinaliza o desligamento gracioso do cliente IPC."""
        info("Sinalizando desligamento do cliente IPC...")
        self.shutdown_event.set() # Define o evento de desligamento
        await self.disconnect() # Garante que a conexão seja fechada


# Funções de log simplificadas (substitua por um sistema de log real em produção)
def info(message: str):
    print(f"[INFO] {message}")

def error(message: str):
    print(f"[ERROR] {message}")

def debug(message: str):
    # Pode ser desativado em produção ou configurado para um nível de log específico
    # print(f"[DEBUG] {message}")
    pass # Desativado por padrão para evitar spam no console durante testes

def warn(message: str): # Adicionando função de aviso
    print(f"[WARN] {message}")


async def main():
    """Exemplo de uso do cliente IPC."""
    ipc_client = IPCClient()

    # Inicia a tarefa de conexão em segundo plano
    connection_task = asyncio.create_task(ipc_client.connect())

    # Inicia a tarefa de recebimento de mensagens em segundo plano
    receive_task = asyncio.create_task(ipc_client.receive_messages_consumer())


    try:
        # Espera um pouco para a conexão ser estabelecida
        await asyncio.sleep(3)

        # --- CORRIGIDO: Construção de mensagens para campos oneof ---
        # Exemplo: Enviar um comando StartComponent
        start_cmd = messages_pb2.Message(
            command=messages_pb2.ControlCommand( # Atribui ao campo 'command' do oneof
                start_component="http_server"
            )
        )
        await ipc_client.send_message(start_cmd)
        await asyncio.sleep(1)

        # Exemplo: Enviar um RequestStatus
        request_status_cmd = messages_pb2.Message(
            command=messages_pb2.ControlCommand( # Atribui ao campo 'command' do oneof
                request_status=True # 'request_status' é um booleano direto
            )
        )
        await ipc_client.send_message(request_status_cmd)
        await asyncio.sleep(2)

        # Exemplo: Enviar um comando Shutdown
        shutdown_cmd = messages_pb2.Message(
            command=messages_pb2.ControlCommand( # Atribui ao campo 'command' do oneof
                shutdown=True # 'shutdown' é um booleano direto
            )
        )
        await ipc_client.send_message(shutdown_cmd)
        info("Comando de shutdown enviado. Aguardando finalização...")
        await asyncio.sleep(5) # Dá um tempo para o servidor Rust e as tarefas encerrarem

    except asyncio.CancelledError:
        info("Execução principal cancelada.")
    finally:
        await ipc_client.shutdown() # Garante que o cliente IPC se desconecte graciosamente
        # Cancela as tarefas de conexão e recebimento se ainda estiverem rodando
        connection_task.cancel()
        receive_task.cancel()
        try:
            await connection_task
        except asyncio.CancelledError:
            pass
        try:
            await receive_task
        except asyncio.CancelledError:
            pass
        info("Cliente IPC de exemplo finalizado.")


if __name__ == "__main__":
    # Garante que o loop de eventos seja executado
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        info("Cliente IPC interrompido manualmente (Ctrl+C).")

