# Esboço para: py_core/src/adapters/wsgi.py

import asyncio
from io import BytesIO

class WSGIAdapter:
    def __init__(self, wsgi_app):
        self.wsgi_app = wsgi_app

    async def handle(self, scope: dict, receive: callable, send: callable):
        # Traduz nosso 'scope' para o dicionário 'environ' do WSGI
        environ = {
            'REQUEST_METHOD': scope['method'],
            'PATH_INFO': scope['path'],
            'SERVER_NAME': '127.0.0.1',
            'SERVER_PORT': str(scope.get('port', 8080)),
            'wsgi.version': (1, 0),
            'wsgi.url_scheme': 'https',
            'wsgi.input': BytesIO(await receive()), # Corpo da requisição
            'wsgi.errors': BytesIO(),
            'wsgi.multithread': True,
            'wsgi.multiprocess': False,
            'wsgi.run_once': False,
        }
        for key, value in scope['headers'].items():
            key = 'HTTP_' + key.upper().replace('-', '_')
            environ[key] = value

        # Define a função start_response que o app WSGI chamará
        response_sent = False
        async def start_response(status, headers):
            nonlocal response_sent
            response_sent = True
            await send({
                'type': 'http.response.start',
                'status': int(status.split(' ')[0]),
                'headers': {k.lower(): v for k, v in headers},
            })

        # Executa o app WSGI síncrono em uma thread separada para não bloquear o event loop
        loop = asyncio.get_running_loop()
        body_iterable = await loop.run_in_executor(
            None,  # Usa o executor de threads padrão
            self.wsgi_app,
            environ,
            start_response
        )

        # Envia o corpo da resposta
        for body_chunk in body_iterable:
            await send({'type': 'http.response.body', 'body': body_chunk})