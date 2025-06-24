import socket
import time
from concurrent.futures import ThreadPoolExecutor

HOST = '127.0.0.1'
PORT = 8080
MESSAGE = b"Hello, Space Core Echo Server!\n" * 4 # 44 bytes de dados

def run_client(client_id, num_requests):
    """Função para um cliente se conectar e enviar/receber dados."""
    total_latency = 0
    successful_requests = 0
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.connect((HOST, PORT))
            s.settimeout(5) # Timeout para operações de socket

            for _ in range(num_requests):
                start_time = time.perf_counter()
                s.sendall(MESSAGE)

                # Lê a resposta. O buffer de 4096 bytes do servidor Rust ecoa
                # o que recebeu, então esperamos o tamanho da mensagem.
                received_data = b""
                while len(received_data) < len(MESSAGE):
                    chunk = s.recv(len(MESSAGE) - len(received_data))
                    if not chunk:
                        print(f"Client {client_id}: Servidor fechou a conexão inesperadamente.")
                        break
                    received_data += chunk

                if received_data == MESSAGE:
                    end_time = time.perf_counter()
                    total_latency += (end_time - start_time)
                    successful_requests += 1
                else:
                    print(f"Client {client_id}: Resposta inválida ou incompleta.")
                    break # Parar se a resposta não for a esperada
    except socket.timeout:
        print(f"Client {client_id}: Conexão com timeout.")
    except ConnectionRefusedError:
        print(f"Client {client_id}: Conexão recusada. O servidor está rodando?")
    except Exception as e:
        print(f"Client {client_id}: Erro inesperado: {e}")

    return successful_requests, total_latency

def main():
    num_clients = 100 # Número de conexões simultâneas
    requests_per_client = 100 # Número de requisições por conexão

    total_successful_requests = 0
    total_time_taken = 0.0

    print(f"Iniciando benchmark com {num_clients} clientes, cada um fazendo {requests_per_client} requisições.")
    print("Certifique-se de que o servidor Space Core está rodando na porta 8080.")

    start_benchmark_time = time.perf_counter()

    with ThreadPoolExecutor(max_workers=num_clients) as executor:
        futures = [executor.submit(run_client, i, requests_per_client) for i in range(num_clients)]

        for future in futures:
            successful_reqs, latency = future.result()
            total_successful_requests += successful_reqs
            total_time_taken += latency # Isso não é ideal para latência total, mas serve para throughput

    end_benchmark_time = time.perf_counter()
    overall_duration = end_benchmark_time - start_benchmark_time

    print("\n--- Resultados do Benchmark ---")
    print(f"Clientes testados: {num_clients}")
    print(f"Requisições por cliente: {requests_per_client}")
    print(f"Total de requisições enviadas (teórico): {num_clients * requests_per_client}")
    print(f"Total de requisições bem-sucedidas: {total_successful_requests}")
    print(f"Duração total do teste: {overall_duration:.2f} segundos")

    if overall_duration > 0 and total_successful_requests > 0:
        requests_per_second = total_successful_requests / overall_duration
        avg_latency_per_request = total_time_taken / total_successful_requests if total_successful_requests > 0 else 0
        print(f"Requisições por segundo (RPS): {requests_per_second:.2f}")
        print(f"Latência média por requisição (Echo): {avg_latency_per_request * 1000:.2f} ms")
    else:
        print("Não foi possível calcular RPS ou latência média (0 requisições bem-sucedidas).")

if __name__ == "__main__":
    main()