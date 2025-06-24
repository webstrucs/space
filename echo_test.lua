-- echo_test.lua
-- Script WRK para testar um servidor echo TCP simples

-- Inicializa a requisição.
-- O 'wrk.method' e 'wrk.path' são para a interface HTTP,
-- mas podemos usá-los para enviar dados brutos se o servidor apenas ecoar.
-- Vamos enviar uma string simples, e o servidor vai ecoar.
-- IMPORTANTE: wrk é projetado para HTTP. Para TCP puro, pode ser necessário
-- enganar o wrk ou usar uma ferramenta mais específica para TCP.
-- No entanto, para fins de teste de "baixa camada" e throughput, podemos
-- simular uma requisição pequena e ver o que acontece.

local request_payload = "hello world\n" -- Payload de exemplo

request = function()
    return wrk.method .. " " .. wrk.path .. " HTTP/1.1\r\n" ..
           "Host: " .. wrk.host .. "\r\n" ..
           "Content-Length: " .. #request_payload .. "\r\n" ..
           "\r\n" ..
           request_payload
end