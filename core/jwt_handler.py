# space/core/jwt_handler.py

import base64
import hmac
import hashlib
import json
import time
from typing import Dict, Any, Optional

# Chave secreta para assinar os tokens. Em produção, use uma variável de ambiente!
JWT_SECRET = "sua-chave-super-secreta-e-longa-e-diferente"

def base64url_encode(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b'=').decode('utf-8')

def base64url_decode(data: str) -> bytes:
    padding = b'=' * (4 - (len(data) % 4))
    return base64.urlsafe_b64decode(data.encode('utf-8') + padding)

def create_token(payload: Dict[str, Any], lifetime_seconds: int = 3600) -> str:
    """Cria um novo token JWT."""
    header = {"alg": "HS256", "typ": "JWT"}
    payload['iat'] = int(time.time())
    payload['exp'] = int(time.time()) + lifetime_seconds
    
    encoded_header = base64url_encode(json.dumps(header, separators=(",", ":")).encode())
    encoded_payload = base64url_encode(json.dumps(payload, separators=(",", ":")).encode())
    
    signature_input = f"{encoded_header}.{encoded_payload}"
    signature = hmac.new(JWT_SECRET.encode(), signature_input.encode(), hashlib.sha256).digest()
    encoded_signature = base64url_encode(signature)
    
    return f"{encoded_header}.{encoded_payload}.{encoded_signature}"

def verify_and_decode_token(token: str) -> Optional[Dict[str, Any]]:
    """Verifica a assinatura e a validade de um token JWT e retorna o payload."""
    try:
        header_b64, payload_b64, signature_b64 = token.split('.')
        
        signature_input = f"{header_b64}.{payload_b64}"
        expected_signature = hmac.new(JWT_SECRET.encode(), signature_input.encode(), hashlib.sha256).digest()
        decoded_signature = base64url_decode(signature_b64)

        if not hmac.compare_digest(decoded_signature, expected_signature):
            print("[JWT] Assinatura inválida!")
            return None

        payload = json.loads(base64url_decode(payload_b64))
        
        if payload['exp'] < int(time.time()):
            print("[JWT] Token expirado!")
            return None
            
        return payload
    except Exception as e:
        print(f"[JWT] Erro ao decodificar/verificar token: {e}")
        return None