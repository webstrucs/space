# space/handlers/route_handlers.py

import json
import mimetypes
from pathlib import Path
from email.utils import formatdate
import secrets

from core.interfaces import Application
from core.http_types import Request, Response
from core.http_response import build_response
from core.templating import render
from core.jwt_handler import create_token, verify_and_decode_token
from core.user_db import get_user_for_login
from core.settings import STATIC_ROOT_PATH

class RootApplication(Application):
    async def handle(self, request: Request) -> Response:
        body = b"Use POST /login para autenticar ou GET /profile com um token."
        return Response(status_code=200, body=body)

class ApiApplication(Application):
    async def handle(self, request: Request) -> Response:
        body = f"Handler de API para o caminho: {request.path}".encode()
        return Response(status_code=200, body=body)

class NotFoundApplication(Application):
    async def handle(self, request: Request) -> Response:
        body = f"404 Not Found: {request.path}".encode()
        return Response(status_code=404, body=body)

class StaticFileApplication(Application):
    async def handle(self, request: Request) -> Response:
        try:
            static_root = STATIC_ROOT_PATH
            relative_path = request.path.removeprefix("/static/").lstrip("/")
            if ".." in Path(relative_path).parts:
                return Response(status_code=403, body=b"Forbidden")
            resolved_path = static_root.joinpath(relative_path).resolve()
            if not resolved_path.is_relative_to(static_root):
                return Response(status_code=403, body=b"Forbidden")
            if resolved_path.is_file():
                with open(resolved_path, "rb") as f: file_body = f.read()
                mime_type, _ = mimetypes.guess_type(resolved_path)
                headers = {"Content-Type": mime_type or "application/octet-stream"}
                headers["Last-Modified"] = formatdate(resolved_path.stat().st_mtime, usegmt=True)
                return Response(status_code=200, headers=headers, body=file_body)
            else:
                return await NotFoundApplication().handle(request)
        except Exception as e:
            return Response(status_code=500, body=f"Internal Server Error: {e}".encode())

class LoginApplication(Application):
    async def handle(self, request: Request) -> Response:
        if request.method.upper() != 'POST':
            return Response(status_code=405, body=b"Method Not Allowed")
        try:
            credentials = json.loads(request.body)
            user_data = get_user_for_login(credentials.get('username'), credentials.get('password'))
            if user_data:
                payload = {"sub": user_data['username'], "role": user_data['role']}
                token = create_token(payload)
                response_body = json.dumps({"token": token}).encode('utf-8')
                return Response(200, headers={"Content-Type": "application/json"}, body=response_body)
            else:
                return Response(401, body=b"Unauthorized: Invalid credentials")
        except Exception:
            return Response(400, body=b"Bad Request: Invalid JSON.")

class ProfileApplication(Application):
    async def handle(self, request: Request) -> Response:
        auth_header = request.headers.get('authorization')
        if not auth_header or not auth_header.lower().startswith('bearer '):
            return Response(401, body=b"Unauthorized: Missing or malformed token.")
        token = auth_header[7:]
        payload = verify_and_decode_token(token)
        if not payload:
            return Response(401, body=b"Unauthorized: Invalid or expired token.")
        context = {"username": payload.get('sub'), "access_level": payload.get('role')}
        html_body = render("profile.html", context)
        return Response(200, headers={"Content-Type": "text/html; charset=utf-8"}, body=html_body)