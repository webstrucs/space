# Conteúdo para: py_core/wfp/minimal_wsgi_app.py
# Um app WSGI simples que não precisa do Flask/Django.

def application(environ, start_response):
    status = '200 OK'
    headers = [('Content-type', 'text/plain; charset=utf-8')]
    start_response(status, headers)

    path = environ.get('PATH_INFO', '/')
    body = f"Ola de um app WSGI para o caminho: {path}".encode('utf-8')

    return [body] # WSGI retorna um iterável de bytes