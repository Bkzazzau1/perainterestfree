# Pera @ quddret.com quick setup notes

This documents the server setup that was provisioned so future devs do not lose context.

## Routing and endpoints
- `pera.quddret.com` → Nginx → `127.0.0.1:8002`
- Payscribe endpoints currently exposed:
  - `POST /api/v1/hooks/payscribe`
  - `GET /payments/payscribe/callback`

## System service
- systemd service: `pera.service`
- Placeholder app path (for now): `/srv/pera_placeholder/app.py`

## Next actions (implementation)
- Replace the placeholder app with the real backend.
- Keep webhook signature verification in mind for `/api/v1/hooks/payscribe`.
- Render or return a simple success/fail response for `/payments/payscribe/callback` (currently JSON).
