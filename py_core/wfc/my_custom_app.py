# Conteúdo para: py_core/wfc/my_custom_app.py
# Este app segue nossa especificação SGI diretamente.

async def application(scope: dict, receive: callable, send: callable):
    body = b"Ola do meu framework customizado!"
    await send({
        'type': 'http.response.start',
        'status': 200,
        'headers': {'content-type': 'text/plain'},
    })
    await send({'type': 'http.response.body', 'body': body})